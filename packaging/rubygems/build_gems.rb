#!/usr/bin/env ruby
# frozen_string_literal: true

# Build multi-platform RubyGems packages wrapping the exact prebuilt `arks`
# binaries emitted by `.github/workflows/release.yml` job `dist`.
#
# Cite: .spec/sot/external-distribution-and-registry-publishing.sot.md

require "fileutils"
require "optparse"
require "rubygems/package"

REPO_ROOT = File.expand_path("../..", __dir__)

TARGETS = {
  "x86_64-unknown-linux-gnu" => {
    archive: "arks-x86_64-unknown-linux-gnu.tar.gz",
    bin_name: "arks",
    platform: "x86_64-linux"
  },
  "aarch64-apple-darwin" => {
    archive: "arks-aarch64-apple-darwin.tar.gz",
    bin_name: "arks",
    platform: "arm64-darwin"
  },
  "x86_64-pc-windows-msvc" => {
    archive: "arks-x86_64-pc-windows-msvc.zip",
    bin_name: "arks.exe",
    platform: "x64-mingw-ucrt"
  }
}.freeze

def read_workspace_version
  cargo = File.read(File.join(REPO_ROOT, "Cargo.toml"))
  match = cargo.match(/^version\s*=\s*"([^"]+)"/)
  raise "FAIL: unable to read workspace version from Cargo.toml" unless match

  match[1]
end

def extract_binary(archive_path, bin_name, dest_path)
  FileUtils.mkdir_p(File.dirname(dest_path))
  py = if archive_path.end_with?(".zip")
         <<~PY
           import pathlib, sys, zipfile
           with zipfile.ZipFile(sys.argv[1]) as zf:
               for name in zf.namelist():
                   if pathlib.Path(name).name == sys.argv[2]:
                       pathlib.Path(sys.argv[3]).write_bytes(zf.read(name))
                       sys.exit(0)
           sys.exit("binary not found in zip")
         PY
       else
         <<~PY
           import pathlib, sys, tarfile
           with tarfile.open(sys.argv[1], "r:gz") as tf:
               for m in tf.getmembers():
                   if m.isfile() and pathlib.Path(m.name).name == sys.argv[2]:
                       pathlib.Path(sys.argv[3]).write_bytes(tf.extractfile(m).read())
                       sys.exit(0)
           sys.exit("binary not found in tar.gz")
         PY
       end
  ok = system("python3", "-c", py, archive_path, bin_name, dest_path)
  raise "FAIL: extraction failed for #{archive_path}" unless ok
end

options = { version: nil, artifacts_dir: nil, out_dir: nil, single_binaries: {} }
OptionParser.new do |opts|
  opts.on("--version VER") { |v| options[:version] = v }
  opts.on("--artifacts-dir DIR") { |v| options[:artifacts_dir] = File.expand_path(v) }
  opts.on("--out-dir DIR") { |v| options[:out_dir] = File.expand_path(v) }
  opts.on("--single-binary SPEC") do |v|
    target, path = v.split("=", 2)
    raise "FAIL: invalid --single-binary #{v}" unless target && path && TARGETS.key?(target)

    options[:single_binaries][target] = File.expand_path(path)
  end
end.parse!

raise "FAIL: --out-dir is required" unless options[:out_dir]

version = options[:version] || read_workspace_version
FileUtils.mkdir_p(options[:out_dir])

built = []

TARGETS.each do |target, cfg|
  stage = File.join(options[:out_dir], ".stage-#{cfg[:platform]}")
  FileUtils.rm_rf(stage)
  FileUtils.mkdir_p(File.join(stage, "exe"))
  FileUtils.mkdir_p(File.join(stage, "libexec"))

  bin_dest = File.join(stage, "libexec", cfg[:bin_name])
  if options[:single_binaries].key?(target)
    FileUtils.cp(options[:single_binaries][target], bin_dest)
  elsif options[:artifacts_dir]
    archive_path = File.join(options[:artifacts_dir], cfg[:archive])
    next unless File.file?(archive_path)

    extract_binary(archive_path, cfg[:bin_name], bin_dest)
  else
    next
  end

  File.chmod(0o755, bin_dest)
  exe_dest = File.join(stage, "exe", "arks")
  FileUtils.cp(File.join(__dir__, "exe", "arks"), exe_dest)
  File.chmod(0o755, exe_dest)

  common_files = []
  %w[LICENSE LICENSE-APACHE LICENSE-MIT README.md].each do |doc|
    src = File.join(REPO_ROOT, doc)
    next unless File.file?(src)

    FileUtils.cp(src, File.join(stage, doc))
    common_files << doc
  end

  spec = Gem::Specification.new do |s|
    s.name = "arks"
    s.version = version
    s.platform = cfg[:platform]
    s.summary = "arks for Git — native Rust Git-compatible CLI, daemon, forge client, and server"
    s.description = "Single native Rust Git-compatible CLI, daemon, independent forge client, and self-hosted server built for multi-session revision contention."
    s.authors = ["Choi Won"]
    s.homepage = "https://modootoday.github.io/arks/"
    s.licenses = ["Apache-2.0", "MIT"]
    s.required_ruby_version = ">= 3.0"
    s.bindir = "exe"
    s.executables = ["arks"]
    s.files = ["exe/arks", "libexec/#{cfg[:bin_name]}"] + common_files
    s.metadata = {
      "homepage_uri" => "https://modootoday.github.io/arks/",
      "source_code_uri" => "https://github.com/modootoday/arks",
      "changelog_uri" => "https://github.com/modootoday/arks/blob/main/CHANGELOG.md",
      "bug_tracker_uri" => "https://github.com/modootoday/arks/issues"
    }
  end

  gem_file = File.join(options[:out_dir], "arks-#{version}-#{cfg[:platform]}.gem")
  Dir.chdir(stage) do
    built_name = Gem::Package.build(spec)
    FileUtils.mv(built_name, gem_file)
  end
  FileUtils.rm_rf(stage)
  built << gem_file
  puts "built #{File.basename(gem_file)} (#{File.size(gem_file)} bytes)"
end

raise "FAIL: no platform binaries found (pass --artifacts-dir or --single-binary)" if built.empty?
