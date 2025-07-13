#!/bin/sh

set -ue

echo "Updating ClamAV configuration..."
# Ensure the config line is not commented out, and update the value
sed -i "s/^.*#.*StreamMaxLength.*/StreamMaxLength ${SCAN_MAX_SIZE_MEGABYTES}M/" /etc/clamav/clamd.conf

# Updating the ClamAV database
echo "Updating ClamAV database..."
freshclam

# Start the ClamAV daemon
echo "Starting ClamAV daemon..."
clamd &


# Switch over to virusscanuser, to reduce the potential attack surface
echo "Switching to non-root user..."
if [ "$(id -u)" -eq 0 ]; then
    echo "Currently running as root, executing as virusscanuser..."
    exec su -s /bin/sh -c "exec /usr/local/bin/virus-scan" virusscanuser
else
    echo "Already running as non-root user, starting directly..."
    exec /usr/local/bin/virus-scan
fi



