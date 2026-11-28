/* Self-hosted progressive enhancement for arks serve Web UI (CSP: script-src 'self'). */
(function () {
  function applyTheme(theme) {
    document.documentElement.setAttribute('data-theme', theme);
    try { localStorage.setItem('arks-theme', theme); } catch (_) {}
  }

  function initTheme() {
    var saved = null;
    try { saved = localStorage.getItem('arks-theme'); } catch (_) {}
    if (saved === 'light' || saved === 'dark') {
      applyTheme(saved);
    }
  }

  function hydrateDiffs() {
    var containers = document.querySelectorAll('[data-diff2html-raw]');
    if (!containers.length || typeof window.Diff2Html === 'undefined') return;
    var mode = 'line-by-line';
    try {
      var saved = localStorage.getItem('arks-diff-mode');
      if (saved === 'side-by-side' || saved === 'line-by-line') mode = saved;
    } catch (_) {}
    containers.forEach(function (el) {
      var rawId = el.getAttribute('data-diff2html-raw');
      var src = rawId ? document.getElementById(rawId) : null;
      if (!src) return;
      var diffText = src.textContent || '';
      if (!diffText.trim()) return;
      try {
        el.innerHTML = window.Diff2Html.html(diffText, {
          drawFileList: true,
          matching: 'lines',
          outputFormat: mode,
          colorScheme: document.documentElement.getAttribute('data-theme') === 'light' ? 'light' : 'dark'
        });
      } catch (_) {}
    });
  }

  function toggleCmdModal(show) {
    var modal = document.getElementById('cmd-modal');
    if (!modal) return;
    if (show) {
      modal.classList.add('open');
      var inp = document.getElementById('cmd-input');
      if (inp) { inp.value = ''; inp.focus(); }
    } else {
      modal.classList.remove('open');
    }
  }

  document.addEventListener('click', function (e) {
    var themeBtn = e.target.closest('[data-action="toggle-theme"]');
    if (themeBtn) {
      var cur = document.documentElement.getAttribute('data-theme') === 'light' ? 'light' : 'dark';
      applyTheme(cur === 'dark' ? 'light' : 'dark');
      hydrateDiffs();
      return;
    }

    var copyBtn = e.target.closest('[data-copy-target]');
    if (copyBtn) {
      var target = document.getElementById(copyBtn.getAttribute('data-copy-target'));
      var txt = target ? target.textContent.trim() : '';
      if (txt && navigator.clipboard) {
        navigator.clipboard.writeText(txt).catch(function () {});
      }
      var orig = copyBtn.textContent;
      copyBtn.textContent = 'Copied';
      setTimeout(function () { copyBtn.textContent = orig; }, 1200);
      return;
    }

    var protoBtn = e.target.closest('[data-clone-cmd]');
    if (protoBtn) {
      var cmdEl = document.getElementById('clone-cmd-text');
      if (cmdEl) cmdEl.textContent = protoBtn.getAttribute('data-clone-cmd');
      document.querySelectorAll('[data-clone-cmd]').forEach(function (b) {
        b.classList.toggle('active', b === protoBtn);
      });
      return;
    }

    var diffModeBtn = e.target.closest('[data-diff-mode]');
    if (diffModeBtn) {
      var mode = diffModeBtn.getAttribute('data-diff-mode');
      try { localStorage.setItem('arks-diff-mode', mode); } catch (_) {}
      document.querySelectorAll('[data-diff-mode]').forEach(function (b) {
        b.classList.toggle('active', b === diffModeBtn);
      });
      var uni = document.getElementById('ssr-diff-unified');
      var spl = document.getElementById('ssr-diff-split');
      if (uni && spl) {
        uni.hidden = mode !== 'line-by-line';
        spl.hidden = mode !== 'side-by-side';
      }
      hydrateDiffs();
      return;
    }

    if (e.target.closest('[data-action="open-cmd"]')) {
      toggleCmdModal(true);
      return;
    }
    if (e.target.id === 'cmd-modal' || e.target.closest('[data-action="close-cmd"]')) {
      toggleCmdModal(false);
      return;
    }

    var ln = e.target.closest('.ln-anchor');
    if (ln) {
      document.querySelectorAll('tr.line-hl').forEach(function (r) { r.classList.remove('line-hl'); });
      var tr = ln.closest('tr');
      if (tr) tr.classList.add('line-hl');
    }
  });

  document.addEventListener('keydown', function (e) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      toggleCmdModal(true);
    } else if (e.key === '/' && document.activeElement && document.activeElement.tagName !== 'INPUT' && document.activeElement.tagName !== 'TEXTAREA') {
      e.preventDefault();
      toggleCmdModal(true);
    } else if (e.key === 'Escape') {
      toggleCmdModal(false);
    }
  });

  document.addEventListener('DOMContentLoaded', function () {
    initTheme();
    hydrateDiffs();
  });

  document.addEventListener('htmx:afterSwap', function () {
    hydrateDiffs();
  });
})();
