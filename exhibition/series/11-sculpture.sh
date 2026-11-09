#!/bin/sh
# Episode 11, Sculpture: the objects shown are the files alea writes.
# This renders the episode's programs to STL, then each STL as a lit solid
# (stl-view.py), next to 11-sculpture.alea, which opens them:
#   11-sculpture.png         render sculpture(f)
#   11-sculpture-at0/1/2.png render sculpture(f, at: 0 / 0.33 / 0.66)
# The STLs and PNGs stay out of the repository.
set -eu
alea=${1:-../../target/release/alea}
tmp=$(mktemp -d)
cat > "$tmp/relief.alea" <<'EOF'
work Relief {
  let f = sin(x * 30 + sin(y * 9) * 2)
  render sculpture(f)
}
EOF
cat > "$tmp/moments.alea" <<'EOF'
work Moments {
  let f = sin(x * 30 + sin(y * 9) * 2 - t * 6)
  render sculpture(f, at: 0)
  render sculpture(f, at: 0.33)
  render sculpture(f, at: 0.66)
}
EOF
"$alea" "$tmp/relief.alea" -o "$tmp" >/dev/null
"$alea" "$tmp/moments.alea" -o "$tmp" >/dev/null
python3 stl-view.py "$tmp"/relief-*.stl 11-sculpture.png >/dev/null
# a work's renders are named in order: moments-<edition>.stl, then -1, -2
base=$(ls "$tmp"/moments-*.stl | grep -v -- '-[0-9]\.stl$')
python3 stl-view.py "$base" 11-sculpture-at0.png >/dev/null
python3 stl-view.py "${base%.stl}-1.stl" 11-sculpture-at1.png >/dev/null
python3 stl-view.py "${base%.stl}-2.stl" 11-sculpture-at2.png >/dev/null
ls "$tmp"/*.stl | head -4
rm -rf "$tmp"
echo "11-sculpture.png, 11-sculpture-at0/1/2.png written"
