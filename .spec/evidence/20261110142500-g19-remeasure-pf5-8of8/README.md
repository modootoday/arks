# G19 remasure — PF5 Claimed 8/8

- Tip / revision: `79fc9bf` (product cut: pack reuse + `arks serve-http` peer from `668642a`)
- Venue: Docker clean-room `arks-verify-m1:0.1.0`, `--cpus=2`, `CARGO_BUILD_JOBS=1`
- J-CLONE-LOCAL-01: git=38 arks=**6** ratio=**0.1579** ≤0.5 PASS
- J-CLONE-PATH-01: git=46 arks=8 ratio=**0.1739** ≤0.5 PASS
- Pf5: claimed=19 aggressive_na=0 miss=[] → **8/8**
- Server profile (pre-cut): git CGI spawn≈0.2ms cgi_wait≈26ms flush≈0; after pack reuse+repack seed POST≈2.7ms
