#!/bin/bash

# Test script for Bash security analysis

# 1. Command injection vulnerabilities
echo "Testing command injection..."

# VULNERABLE: eval with user input
user_input="$1"
eval "echo $user_input"  # Should trigger bash.command-injection

# VULNERABLE: Unsafe user input in commands
rm $2  # Should trigger bash.unsafe-user-input

# VULNERABLE: Using command line arguments unsafely
cat $@  # Should trigger bash.unsafe-user-input

# SAFE: Properly validated input
if [[ "$user_input" =~ ^[a-zA-Z0-9_]+$ ]]; then
    eval "echo $user_input"  # Should NOT trigger
fi

# 2. Hardcoded credentials
echo "Testing hardcoded credentials..."

# VULNERABLE: Hardcoded password
PASSWORD="supersecret123"  # Should trigger bash.hardcoded-credentials
API_KEY="sk-1234567890abcdef"  # Should trigger bash.hardcoded-credentials

# SAFE: Environment variable
PASSWORD="${DB_PASSWORD}"  # Should NOT trigger

# 3. Unsafe temporary files
echo "Testing temporary file creation..."

# VULNERABLE: Predictable temp file
temp_file="/tmp/myapp_temp"  # Should trigger bash.unsafe-temp-file
echo "data" > /var/tmp/backup  # Should trigger bash.unsafe-temp-file

# SAFE: Using mktemp
TEMP=$(mktemp)
echo "data" > "$TEMP"  # Should NOT trigger

# 4. Insecure curl usage
echo "Testing curl security..."

# VULNERABLE: curl with insecure flag
curl -k https://example.com/api  # Should trigger bash.curl-without-verification
curl --insecure https://example.com/data  # Should trigger bash.curl-without-verification

# SAFE: curl with proper verification
curl --cacert /path/to/cert https://example.com/api  # Should NOT trigger

# 5. World-writable files
echo "Testing file permissions..."

# VULNERABLE: World-writable permissions
chmod 777 /tmp/shared_file  # Should trigger bash.world-writable-file
chmod a+w important_file  # Should trigger bash.world-writable-file
chmod o+w config.txt  # Should trigger bash.world-writable-file

# SAFE: Proper permissions
chmod 644 config.txt  # Should NOT trigger

# 6. Sudo without password
echo "Testing sudo usage..."

# VULNERABLE: sudo without password check
sudo -n systemctl restart service  # Should trigger bash.sudo-without-password

# SAFE: sudo with password check
if sudo -n true 2>/dev/null; then
    sudo -n systemctl restart service  # Should NOT trigger
fi

# 7. Unquoted variables
echo "Testing variable quoting..."

# VULNERABLE: Unquoted variables
file_list=$HOME/files/*  # Should trigger bash.unquoted-variables
echo $file_list  # Should trigger bash.unquoted-variables

# SAFE: Quoted variables
file_list="$HOME/files/*"  # Should NOT trigger
echo "$file_list"  # Should NOT trigger

# 8. Dangerous rm commands
echo "Testing dangerous rm commands..."

# VULNERABLE: Dangerous rm usage
# rm -rf /  # Should trigger bash.dangerous-rm-command (commented for safety)
# rm -rf /*  # Should trigger bash.dangerous-rm-command (commented for safety)

# SAFE: rm with confirmation
if [[ "$CONFIRM" == "yes" ]]; then
    rm -rf /tmp/safe_to_delete  # Should NOT trigger
fi

# 9. Shell injection via backticks
echo "Testing backtick injection..."

# VULNERABLE: Command injection via backticks
user_command="$3"
result=`$user_command`  # Should trigger bash.shell-injection-via-backticks

# SAFE: Validated command
if [[ "$user_command" =~ ^[a-zA-Z0-9_/-]+$ ]]; then
    result=`$user_command`  # Should NOT trigger
fi

# 10. Function definitions
function safe_function() {
    local input="$1"
    echo "Processing: $input"
}

function unsafe_function() {
    eval "$1"  # Should trigger bash.command-injection
}

# 11. Loops and conditionals
for file in *.txt; do
    if [[ -f "$file" ]]; then
        cat "$file"
    fi
done

while read -r line; do
    echo "Line: $line"
done < input.txt

# 12. Case statements
case "$1" in
    start)
        echo "Starting service..."
        ;;
    stop)
        echo "Stopping service..."
        ;;
    *)
        echo "Unknown command: $1"
        ;;
esac

# 13. Array usage
declare -a files=("file1.txt" "file2.txt" "file3.txt")
for file in "${files[@]}"; do
    echo "Processing: $file"
done

# 14. Here documents
cat << EOF
This is a here document
with multiple lines
EOF

# 15. Process substitution
diff <(sort file1.txt) <(sort file2.txt)

# 16. Command substitution
current_date=$(date)
echo "Current date: $current_date"

# 17. Arithmetic expansion
count=$((count + 1))
echo "Count: $count"

# 18. Parameter expansion
filename="${1:-default.txt}"
echo "Filename: $filename"

# 19. Exit codes
if command -v git >/dev/null 2>&1; then
    echo "Git is installed"
else
    echo "Git is not installed"
    exit 1
fi

echo "Bash security test completed."
