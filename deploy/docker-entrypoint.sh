#!/bin/sh
set -eu

port=${SERVER_PORT:-25565}
case "$port" in
    ''|*[!0-9]*)
        printf '%s\n' 'SERVER_PORT must be an integer in 1..65535.' >&2
        exit 64
        ;;
esac

# Normalize leading zeros before numeric comparison and bound the input length
# before asking the shell to parse it, including hostile very large values.
while [ "${port#0}" != "$port" ]; do
    port=${port#0}
done
if [ -z "$port" ] || [ "${#port}" -gt 5 ] || [ "$port" -gt 65535 ]; then
    printf '%s\n' 'SERVER_PORT must be an integer in 1..65535.' >&2
    exit 64
fi

# Wings supplies STARTUP as an environment variable. This image intentionally
# uses the fixed native startup below; Docker arguments remain separate argv.
exec /usr/local/bin/ferrumc --bind "0.0.0.0:$port" "$@"
