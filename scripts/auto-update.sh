#!/bin/bash
# ════════════════════════════════════════════════════════════════════════════
# Network Ghost v5.0 - اسکریپت به‌روزرسانی خودکار
# این فایل توسط Systemd Timer فراخوانی می‌شود
# ════════════════════════════════════════════════════════════════════════════

set -euo pipefail

INSTALL_DIR="/opt/network-ghost"
LOG_DIR="${INSTALL_DIR}/logs"
LOG_FILE="${LOG_DIR}/auto-update-$(date +%Y%m%d).log"
MAX_LOG_DAYS=7
LOCK_FILE="/tmp/network-ghost-update.lock"

mkdir -p "$LOG_DIR"

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] [INFO] $1" | tee -a "$LOG_FILE"; }
error() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] [ERROR] $1" | tee -a "$LOG_FILE" >&2; }

# ─── قفل فایل (جلوگیری از تداخل) ───────────────────────────────────────────
if [[ -f "$LOCK_FILE" ]]; then
    LOCK_AGE=$(( $(date +%s) - $(stat -c %Y "$LOCK_FILE" 2>/dev/null || echo 0) ))
    if [[ $LOCK_AGE -lt 660 ]]; then
        log "نمونه دیگری در حال اجراست (lock age: ${LOCK_AGE}s). خروج."
        exit 0
    fi
    log "قفل قدیمی (${LOCK_AGE}s) حذف می‌شود."
fi
touch "$LOCK_FILE"
trap 'rm -f "$LOCK_FILE"' EXIT
# ────────────────────────────────────────────────────────────────────────────

log "════════ شروع به‌روزرسانی خودکار Network Ghost ════════"

# ─── پاکسازی لاگ‌های قدیمی ──────────────────────────────────────────────────
find "$LOG_DIR" -name "auto-update-*.log" -mtime "+${MAX_LOG_DAYS}" -delete 2>/dev/null || true
log "لاگ‌های قدیمی‌تر از ${MAX_LOG_DAYS} روز پاکسازی شدند."
# ────────────────────────────────────────────────────────────────────────────

# ─── بررسی اتصال اینترنت ────────────────────────────────────────────────────
check_connectivity() {
    for endpoint in "1.1.1.1" "8.8.8.8" "9.9.9.9"; do
        if ping -c 1 -W 3 "$endpoint" &>/dev/null; then
            return 0
        fi
    done
    return 1
}

if ! check_connectivity; then
    error "اتصال اینترنت برقرار نیست. بررسی لغو شد."
    exit 1
fi
log "اتصال اینترنت: OK"
# ────────────────────────────────────────────────────────────────────────────

# ─── اجرای proxy-checker ────────────────────────────────────────────────────
CHECKER="${INSTALL_DIR}/proxy-checker"
if [[ ! -x "$CHECKER" ]]; then
    error "فایل اجرایی proxy-checker یافت نشد: $CHECKER"
    exit 1
fi

log "شروع اسکن پروکسی‌ها..."
START_TIME=$(date +%s)

"$CHECKER" \
    --proxy-file "${INSTALL_DIR}/edge/assets/p-list-february.txt" \
    --output-file "${INSTALL_DIR}/sub/ProxyIP-Daily.md" \
    --json-output "${INSTALL_DIR}/sub/ProxyIP-Daily.json" \
    --max-concurrent 30 \
    --timeout 8 \
    2>&1 | tee -a "$LOG_FILE"

END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))
# ────────────────────────────────────────────────────────────────────────────

# ─── آمار خروجی ─────────────────────────────────────────────────────────────
if [[ -f "${INSTALL_DIR}/sub/ProxyIP-Daily.md" ]]; then
    PROXY_COUNT=$(grep -c "| \`" "${INSTALL_DIR}/sub/ProxyIP-Daily.md" 2>/dev/null || echo 0)
    log "✅ به‌روزرسانی موفق: ${PROXY_COUNT} پروکسی فعال یافت شد (${ELAPSED}s)"
    echo "$PROXY_COUNT" > "${LOG_DIR}/last-proxy-count.txt"
    date '+%Y-%m-%d %H:%M:%S' > "${LOG_DIR}/last-success.txt"
else
    error "فایل خروجی ایجاد نشد."
    exit 1
fi

log "════════ پایان به‌روزرسانی خودکار Network Ghost ════════"
