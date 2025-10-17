#!/usr/bin/env python3
"""
Performance benchmark for Bash and SQL language support
"""

import subprocess
import time
import json
import os
import sys
from pathlib import Path

def run_analysis(rule_file, target_file):
    """Run analysis and measure performance"""
    start_time = time.time()
    
    cmd = [
        "./target/debug/cr-semservice",
        "analyze",
        "--config", rule_file,
        target_file
    ]
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, cwd=".")
        
        end_time = time.time()
        execution_time = (end_time - start_time) * 1000  # Convert to milliseconds
        
        if result.returncode != 0:
            print(f"Error running analysis: {result.stderr}")
            return None, execution_time
        
        # Extract JSON from output
        stdout = result.stdout
        json_start = stdout.find('{')
        json_end = stdout.rfind('}') + 1
        
        if json_start == -1 or json_end == 0:
            print("No JSON found in output")
            return None, execution_time
        
        json_str = stdout[json_start:json_end]
        analysis_result = json.loads(json_str)
        
        return analysis_result, execution_time
        
    except Exception as e:
        end_time = time.time()
        execution_time = (end_time - start_time) * 1000
        print(f"Exception running analysis: {e}")
        return None, execution_time

def create_large_bash_file(filename, size_multiplier=10):
    """Create a large Bash file for performance testing"""
    base_content = '''#!/bin/bash

# Function definitions
function process_data() {
    local input="$1"
    local output="$2"
    
    # VULNERABLE: eval with user input
    eval "echo $input"
    
    # VULNERABLE: Unquoted variables
    cat $input > $output
    
    # VULNERABLE: Hardcoded credentials
    PASSWORD="secret123"
    API_KEY="sk-abcdef123456"
    
    # VULNERABLE: Unsafe temp files
    temp_file="/tmp/process_$RANDOM"
    echo "data" > $temp_file
    
    # VULNERABLE: World writable files
    chmod 777 $output
    
    # VULNERABLE: Command injection via backticks
    result=`$input`
    
    return 0
}

# Main processing loop
for file in *.txt; do
    if [[ -f "$file" ]]; then
        # VULNERABLE: Unsafe user input
        rm $1
        
        # VULNERABLE: curl without verification
        curl -k https://api.example.com/upload
        
        # VULNERABLE: sudo without password
        sudo -n systemctl restart service
        
        process_data "$file" "output_$file"
    fi
done

# Conditional processing
if [[ "$1" == "dangerous" ]]; then
    # VULNERABLE: Dangerous rm command
    # rm -rf /tmp/*
    echo "Dangerous operation requested"
fi

# Case statement
case "$2" in
    start)
        echo "Starting service..."
        ;;
    stop)
        echo "Stopping service..."
        ;;
    *)
        echo "Unknown command: $2"
        ;;
esac

# Array processing
declare -a files=("file1.sh" "file2.sh" "file3.sh")
for script in "${files[@]}"; do
    # VULNERABLE: eval in loop
    eval "source $script"
done

# Here document
cat << EOF > config.txt
# Configuration file
PASSWORD=hardcoded123
SECRET_KEY=abc123def456
EOF

# Process substitution
diff <(sort file1.txt) <(sort file2.txt)

# Command substitution
current_date=$(date)
echo "Processing started at: $current_date"

# Arithmetic operations
count=0
while [[ $count -lt 100 ]]; do
    count=$((count + 1))
    echo "Processing item $count"
done

echo "Bash script completed."
'''
    
    with open(filename, 'w') as f:
        for i in range(size_multiplier):
            f.write(f"# Section {i+1}\n")
            f.write(base_content)
            f.write(f"\n# End of section {i+1}\n\n")

def create_large_sql_file(filename, size_multiplier=10):
    """Create a large SQL file for performance testing"""
    base_content = '''-- Database operations

-- VULNERABLE: SQL injection
SELECT * FROM users WHERE username = 'admin' + @input;
SELECT * FROM products WHERE id = 1 OR 1=1;

-- VULNERABLE: UNION injection
SELECT name FROM items UNION SELECT password FROM users;

-- VULNERABLE: Hardcoded passwords
CREATE USER 'testuser'@'localhost' IDENTIFIED BY 'password123';
ALTER USER 'admin'@'%' IDENTIFIED BY 'supersecret';

-- VULNERABLE: SELECT * without limits
SELECT * FROM sensitive_data;
SELECT * FROM user_profiles;

-- VULNERABLE: Missing WHERE clauses
DELETE FROM temp_table;
UPDATE user_settings SET active = 0;

-- VULNERABLE: Privilege escalation
GRANT ALL ON *.* TO 'webapp'@'%';
GRANT ALL PRIVILEGES ON *.* TO 'service'@'localhost';

-- VULNERABLE: Weak encryption
SELECT MD5(password) FROM users;
SELECT SHA1(sensitive_data) FROM records;
SELECT DES_ENCRYPT(credit_card, 'key') FROM payments;

-- VULNERABLE: Information disclosure
SELECT USER();
SELECT VERSION();
SELECT DATABASE();
SHOW DATABASES;
SHOW TABLES;

-- VULNERABLE: Time-based attacks
SELECT * FROM users WHERE id = 1 AND SLEEP(5);
SELECT * FROM data WHERE BENCHMARK(1000000, MD5('test'));

-- VULNERABLE: File operations
SELECT LOAD_FILE('/etc/passwd');
SELECT * FROM users INTO OUTFILE '/tmp/users.txt';

-- VULNERABLE: Command execution
EXEC xp_cmdshell 'dir';

-- Complex queries
SELECT u.username, p.product_name, o.order_date
FROM users u
JOIN orders o ON u.id = o.user_id
JOIN products p ON o.product_id = p.id
WHERE u.active = 1
AND o.order_date > '2023-01-01'
ORDER BY o.order_date DESC;

-- Stored procedures
DELIMITER //
CREATE PROCEDURE GetUserData(IN user_id INT)
BEGIN
    SET @sql = CONCAT('SELECT * FROM users WHERE id = ', user_id);
    PREPARE stmt FROM @sql;
    EXECUTE stmt;
    DEALLOCATE PREPARE stmt;
END //
DELIMITER ;

-- Triggers
CREATE TRIGGER user_audit 
AFTER INSERT ON users 
FOR EACH ROW
BEGIN
    INSERT INTO audit_log (action, user_id, password) 
    VALUES ('INSERT', NEW.id, 'audit_password123');
END;

-- Views
CREATE VIEW user_summary AS
SELECT username, email, MD5(password) as password_hash
FROM users 
WHERE active = 1;

-- Transactions
START TRANSACTION;
UPDATE accounts SET balance = balance - 100 WHERE id = 1;
UPDATE accounts SET balance = balance + 100 WHERE id = 2;
COMMIT;

-- End of SQL section
'''
    
    with open(filename, 'w') as f:
        for i in range(size_multiplier):
            f.write(f"-- Section {i+1}\n")
            f.write(base_content)
            f.write(f"\n-- End of section {i+1}\n\n")

def benchmark_bash_sql_performance():
    """Benchmark Bash and SQL performance"""
    
    # Create test files of different sizes
    bash_files = []
    sql_files = []
    
    for size in [1, 5, 10, 20]:
        bash_filename = f"tests/bash-sql/perf_bash_{size}x.sh"
        sql_filename = f"tests/bash-sql/perf_sql_{size}x.sql"
        
        create_large_bash_file(bash_filename, size)
        create_large_sql_file(sql_filename, size)
        
        bash_files.append((bash_filename, size))
        sql_files.append((sql_filename, size))
    
    # Test configurations
    test_configs = [
        ("tests/bash-sql/bash_security_rules.yaml", bash_files, "Bash"),
        ("tests/bash-sql/sql_security_rules.yaml", sql_files, "SQL"),
    ]
    
    results = []
    
    print("🚀 Starting Bash and SQL performance benchmark...")
    print("="*60)
    
    for config_file, test_files, language in test_configs:
        if not os.path.exists(config_file):
            print(f"⚠️  Skipping {language}: {config_file} not found")
            continue
            
        print(f"\n📊 Testing: {language}")
        print("-" * 40)
        
        config_results = []
        
        for test_file, size_multiplier in test_files:
            print(f"  📁 File size: {size_multiplier}x baseline", end=" ... ")
            
            # Run multiple iterations for more accurate timing
            times = []
            findings_counts = []
            
            for iteration in range(3):
                analysis_result, execution_time = run_analysis(config_file, test_file)
                
                if analysis_result:
                    times.append(execution_time)
                    findings_counts.append(analysis_result.get("summary", {}).get("total_findings", 0))
                else:
                    print(f"❌ Failed iteration {iteration + 1}")
            
            if times:
                avg_time = sum(times) / len(times)
                avg_findings = sum(findings_counts) / len(findings_counts) if findings_counts else 0
                
                print(f"⏱️  {avg_time:.1f}ms (avg), 🔍 {avg_findings:.0f} findings")
                
                config_results.append({
                    "size_multiplier": size_multiplier,
                    "avg_time_ms": avg_time,
                    "avg_findings": avg_findings,
                    "times": times
                })
            else:
                print("❌ All iterations failed")
        
        results.append({
            "language": language,
            "config_file": config_file,
            "results": config_results
        })
    
    # Clean up test files
    for test_file, _ in bash_files + sql_files:
        if os.path.exists(test_file):
            os.remove(test_file)
    
    return results

def print_performance_summary(results):
    """Print performance summary and analysis"""
    print("\n" + "="*60)
    print("🎯 BASH AND SQL PERFORMANCE BENCHMARK SUMMARY")
    print("="*60)
    
    if not results:
        print("❌ No results to analyze")
        return
    
    # Calculate performance metrics
    for result in results:
        language = result["language"]
        config_results = result["results"]
        
        if not config_results:
            continue
            
        print(f"\n📈 {language} Performance:")
        print("-" * 40)
        
        # Calculate scaling factor
        baseline_time = None
        for res in config_results:
            if res["size_multiplier"] == 1:
                baseline_time = res["avg_time_ms"]
                break
        
        if baseline_time:
            print(f"  📊 Performance scaling:")
            for res in config_results:
                size = res["size_multiplier"]
                time_ms = res["avg_time_ms"]
                findings = res["avg_findings"]
                
                scaling_factor = time_ms / baseline_time if baseline_time > 0 else 0
                efficiency = findings / time_ms if time_ms > 0 else 0
                
                print(f"    {size:2d}x size: {time_ms:6.1f}ms ({scaling_factor:4.1f}x time), "
                      f"{findings:3.0f} findings, {efficiency:5.2f} findings/ms")
        
        # Performance assessment
        largest_result = max(config_results, key=lambda x: x["size_multiplier"])
        if largest_result["avg_time_ms"] < 1000:  # Less than 1 second
            print(f"  ✅ Performance: EXCELLENT (max {largest_result['avg_time_ms']:.1f}ms)")
        elif largest_result["avg_time_ms"] < 5000:  # Less than 5 seconds
            print(f"  ✅ Performance: GOOD (max {largest_result['avg_time_ms']:.1f}ms)")
        else:
            print(f"  ⚠️  Performance: NEEDS OPTIMIZATION (max {largest_result['avg_time_ms']:.1f}ms)")
    
    # Overall assessment
    print(f"\n🏆 OVERALL ASSESSMENT:")
    print("-" * 40)
    
    max_time_across_all = 0
    total_languages_tested = 0
    
    for result in results:
        if result["results"]:
            total_languages_tested += 1
            language_max = max(res["avg_time_ms"] for res in result["results"])
            max_time_across_all = max(max_time_across_all, language_max)
    
    if total_languages_tested == 0:
        print("❌ No languages successfully tested")
    elif max_time_across_all < 1000:
        print(f"✅ EXCELLENT: Both Bash and SQL perform well (max {max_time_across_all:.1f}ms)")
        print("🚀 Tree-sitter integration maintains excellent performance!")
    elif max_time_across_all < 5000:
        print(f"✅ GOOD: Bash and SQL have acceptable performance (max {max_time_across_all:.1f}ms)")
        print("⚡ Performance is within acceptable limits")
    else:
        print(f"⚠️  NEEDS WORK: Some features may need optimization (max {max_time_across_all:.1f}ms)")
        print("🔧 Consider optimizing slow patterns or implementations")

if __name__ == "__main__":
    print("🔬 CR-SemService Bash and SQL Performance Benchmark")
    print("="*60)
    
    # Check if binary exists
    if not os.path.exists("./target/debug/cr-semservice"):
        print("❌ Binary not found. Please build the project first:")
        print("   cargo build")
        sys.exit(1)
    
    try:
        results = benchmark_bash_sql_performance()
        print_performance_summary(results)
        
    except Exception as e:
        print(f"\n❌ Benchmark failed: {e}")
        sys.exit(1)
