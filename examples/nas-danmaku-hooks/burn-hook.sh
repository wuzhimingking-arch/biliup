#!/bin/sh
# biliup segment_processor: receives video path and optional XML path on stdin.
# This script runs INSIDE one of the two existing biliup containers.
set -eu

payload=$(cat)
video=$(printf '%s\n' "$payload" | sed -n '1p')
xml=$(printf '%s\n' "$payload" | sed -n '2p')
# Upstream may pass relative paths. Resolve them against the process working directory.
# Only canonical paths inside the existing /opt data volume are permitted.
[ -n "$video" ] || { echo "[burn] missing video path" >&2; exit 61; }
[ -n "$xml" ] || { echo "[burn] XML missing in segment event; enable danmaku recording" >&2; exit 62; }
video=$(readlink -f "$video") || { echo "[burn] missing video: $video" >&2; exit 63; }
xml=$(readlink -f "$xml") || { echo "[burn] missing XML: $xml" >&2; exit 64; }
case "$video" in /opt/*.flv) ;; *) echo "[burn] unsupported video path: $video" >&2; exit 65;; esac
case "$xml" in /opt/*.xml) ;; *) echo "[burn] unsupported XML path: $xml" >&2; exit 66;; esac
[ -f "$video" ] || { echo "[burn] finalized video not found: $video" >&2; exit 67; }
[ -f "$xml" ] || { echo "[burn] finalized XML not found: $xml" >&2; exit 68; }

q=/opt/.burn-queue
mkdir -p "$q/pending" "$q/working" "$q/results" "$q/logs"
id=$(cat /proc/sys/kernel/random/uuid)
file="$q/pending/$id.job"
# Atomic request publishing, so the worker never reads a half-written job.
printf '%s\n%s\n' "$video" "$xml" > "$file.tmp"
mv "$file.tmp" "$file"
echo "[burn] queued $id; waiting for worker (large videos can take hours)" >&2

# Keep the existing encoding deadline; an unclaimed job can be withdrawn safely.
count=0
while [ "$count" -lt 86400 ]; do
  if [ -f "$q/results/$id.ok" ]; then
    echo "[burn] $id completed; biliup may upload the burned FLV" >&2
    exit 0
  fi
  if [ -f "$q/results/$id.err" ]; then
    echo "[burn] ERROR $id: $(cat "$q/results/$id.err")" >&2
    echo "[burn] see NAS .burn-queue/logs/$id.log and original files" >&2
    exit 70
  fi
  # Atomically withdraw only an unclaimed request; never race a running encoder.
  if [ "$count" -ge 60 ] && [ -f "$file" ]; then
    if mv "$file" "$q/results/$id.cancelled" 2>/dev/null; then
      echo "[burn] queue not claimed in 120 seconds; original retained" >&2
      exit 72
    fi
  fi
  sleep 2
  count=$((count+1))
done
printf '%s\n' "[burn] TIMED OUT; original video retained; manual recovery required" >&2
exit 71
