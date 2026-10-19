#!/bin/sh
# Biliup supplies finalized video/XML paths on stdin. A failed burn may use a healthy original.
set -u
payload=$(cat)
video=$(printf '%s\n' "$payload" | sed -n '1p')
xml=$(printf '%s\n' "$payload" | sed -n '2p')
[ -n "$video" ] && [ ! -L "$video" ] || exit 81
video=$(readlink -f "$video") || exit 82
case "$video" in /opt/*.flv) ;; *) exit 83;; esac
[ -s "$video" ] && [ -f "$video" ] || exit 84

healthy_video() {
  # shortcut: bounded head/tail decode; use a full offline scan for historical corruption audits.
  timeout 30 ffprobe -v error -select_streams v:0 -show_entries stream=codec_name -of csv=p=0 "$video" | grep -Eq '^[a-zA-Z0-9_]+' || return 1
  timeout 30 ffmpeg -nostdin -v error -xerror -i "$video" -t 2 -map 0:v:0 -map '0:a?' -f null - || return 1
  timeout 30 ffmpeg -nostdin -v error -xerror -sseof -2 -i "$video" -t 2 -map 0:v:0 -map '0:a?' -f null - || return 1
}
fallback() {
  healthy_video || { echo '[burn-safe] original video failed validation; upload blocked' >&2; exit 85; }
  mkdir -p /opt/.burn-queue
  printf '[%s] %s; original video retained and validated\n' "$(date '+%F %T')" "$1" >> /opt/.burn-queue/fail-open.log
  echo '[burn-safe] using validated original video' >&2
  exit 0
}

if [ -z "$xml" ] || [ ! -s "$xml" ]; then fallback 'XML missing or empty'; fi
[ ! -L "$xml" ] || fallback 'XML symlink rejected'
xml=$(readlink -f "$xml") || fallback 'XML path invalid'
case "$xml" in /opt/*.xml) ;; *) fallback 'XML outside recording volume';; esac
if ! grep -Eq '<d[[:space:]>]' "$xml" || ! tail -c 128 "$xml" | grep -q '</i>'; then
  fallback 'XML has no real chat records or is not closed'
fi
printf '%s\n' "$payload" | sh /opt/burn-hook.sh
rc=$?
# A claimed encoder may still own the file at its deadline; never race it with upload.
if [ "$rc" -eq 71 ]; then echo '[burn-safe] encoder deadline reached; manual recovery required' >&2; exit 71; fi
if [ "$rc" -ne 0 ]; then fallback "burn-hook failed rc=$rc"; fi
healthy_video || { echo '[burn-safe] processed video failed validation' >&2; exit 86; }
exit 0
