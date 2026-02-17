-- Information Disclosure Test Cases
-- This file contains examples of information gathering queries

-- ============================================================================
-- 1. System Information Functions
-- ============================================================================

-- VULNERABLE: SELECT USER()
-- ruleid: information-disclosure-001
SELECT USER();

-- VULNERABLE: SELECT VERSION()
-- ruleid: information-disclosure-001
SELECT VERSION();

-- VULNERABLE: SELECT DATABASE()
-- ruleid: information-disclosure-001
SELECT DATABASE();

-- VULNERABLE: SELECT CURRENT_USER()
-- ruleid: information-disclosure-001
SELECT CURRENT_USER();

-- VULNERABLE: SELECT CURRENT_DATABASE()
-- ruleid: information-disclosure-001
SELECT CURRENT_DATABASE();

-- SAFE: Application-controlled information
SELECT 'Application v1.0' as version;

-- SAFE: Hardcoded safe values
SELECT 'production' as environment;

-- ============================================================================
-- 2. SHOW Commands
-- ============================================================================

-- VULNERABLE: SHOW DATABASES
-- ruleid: information-disclosure-002
SHOW DATABASES;

-- VULNERABLE: SHOW TABLES
-- ruleid: information-disclosure-002
SHOW TABLES;

-- VULNERABLE: SHOW SCHEMAS
-- ruleid: information-disclosure-002
SHOW SCHEMAS;

-- VULNERABLE: SHOW COLUMNS
-- ruleid: information-disclosure-002
SHOW COLUMNS FROM users;

-- VULNERABLE: SHOW FULL TABLES
-- ruleid: information-disclosure-002
SHOW FULL TABLES;

-- VULNERABLE: SHOW TABLE STATUS
-- ruleid: information-disclosure-002
SHOW TABLE STATUS;

-- VULNERABLE: SHOW VARIABLES
-- ruleid: information-disclosure-002
SHOW VARIABLES;

-- VULNERABLE: SHOW GLOBAL VARIABLES
-- ruleid: information-disclosure-002
SHOW GLOBAL VARIABLES;

-- SAFE: Query information schema with restrictions
SELECT table_name FROM information_schema.tables WHERE table_schema = 'myapp';

-- ============================================================================
-- 3. Information Schema Queries
-- ============================================================================

-- VULNERABLE: Query all databases
-- ruleid: information-disclosure-003
SELECT * FROM information_schema.schemata;

-- VULNERABLE: Query all tables
-- ruleid: information-disclosure-003
SELECT * FROM information_schema.tables;

-- VULNERABLE: Query all columns
-- ruleid: information-disclosure-003
SELECT * FROM information_schema.columns;

-- VULNERABLE: Query all users
-- ruleid: information-disclosure-003
SELECT * FROM information_schema.user_privileges;

-- VULNERABLE: Query all processes
-- ruleid: information-disclosure-003
SELECT * FROM information_schema.processlist;

-- SAFE: Query specific database tables
SELECT table_name FROM information_schema.tables WHERE table_schema = 'myapp';

-- SAFE: Query specific columns
SELECT column_name FROM information_schema.columns WHERE table_schema = 'myapp' AND table_name = 'users';

-- ============================================================================
-- 4. System Catalog Queries
-- ============================================================================

-- VULNERABLE: Query system tables
-- ruleid: information-disclosure-004
SELECT * FROM mysql.user;

-- VULNERABLE: Query password hashes
-- ruleid: information-disclosure-004
SELECT user, authentication_string FROM mysql.user;

-- VULNERABLE: Query privileges
-- ruleid: information-disclosure-004
SELECT * FROM mysql.db;

-- VULNERABLE: Query grants
-- ruleid: information-disclosure-004
SELECT * FROM mysql.tables_priv;

-- SAFE: Use SHOW GRANTS for current user
SHOW GRANTS FOR CURRENT_USER();

-- ============================================================================
-- 5. File System Information
-- ============================================================================

-- VULNERABLE: LOAD_FILE to read files
-- ruleid: information-disclosure-005
SELECT LOAD_FILE('/etc/passwd');

-- VULNERABLE: LOAD_FILE for configuration
-- ruleid: information-disclosure-005
SELECT LOAD_FILE('/etc/mysql/my.cnf');

-- VULNERABLE: INTO OUTFILE to write files
-- ruleid: information-disclosure-005
SELECT * FROM users INTO OUTFILE '/tmp/users.txt';

-- VULNERABLE: INTO DUMPFILE
-- ruleid: information-disclosure-005
SELECT password INTO DUMPFILE '/tmp/passwords.txt' FROM users;

-- SAFE: Use application-level file handling
-- (No SQL file operations)

-- ============================================================================
-- 6. Error-based Information Disclosure
-- ============================================================================

-- VULNERABLE: Intentional error to extract info
-- ruleid: information-disclosure-006
SELECT * FROM users WHERE id = 1 AND extractvalue(1, concat(0x7e, (SELECT version())));

-- VULNERABLE: UpdateXML error
-- ruleid: information-disclosure-006
SELECT * FROM users WHERE id = 1 AND updatexml(1, concat(0x7e, (SELECT user())), 1);

-- VULNERABLE: JSON error
-- ruleid: information-disclosure-006
SELECT * FROM users WHERE id = 1 AND json_extract(1, concat(0x7e, (SELECT database())));

-- SAFE: Proper error handling
SELECT * FROM users WHERE id = 1;

-- ============================================================================
-- 7. Timing-based Information Disclosure
-- ============================================================================

-- VULNERABLE: SLEEP to extract information
-- ruleid: information-disclosure-007
SELECT * FROM users WHERE id = 1 AND IF(version() LIKE '5%', SLEEP(5), 0);

-- VULNERABLE: BENCHMARK for timing
-- ruleid: information-disclosure-007
SELECT * FROM users WHERE id = 1 AND BENCHMARK(1000000, MD5(database()));

-- VULNERABLE: WAITFOR for timing
-- ruleid: information-disclosure-007
SELECT * FROM users WHERE id = 1 AND WAITFOR DELAY '00:00:05';

-- SAFE: Normal query without timing
SELECT * FROM users WHERE id = 1;

-- ============================================================================
-- 8. Metadata Queries
-- ============================================================================

-- VULNERABLE: Query table metadata
-- ruleid: information-disclosure-008
SELECT * FROM information_schema.statistics;

-- VULNERABLE: Query key information
-- ruleid: information-disclosure-008
SELECT * FROM information_schema.key_column_usage;

-- VULNERABLE: Query constraint information
-- ruleid: information-disclosure-008
SELECT * FROM information_schema.table_constraints;

-- VULNERABLE: Query trigger information
-- ruleid: information-disclosure-008
SELECT * FROM information_schema.triggers;

-- SAFE: Query only necessary metadata
SELECT column_name FROM information_schema.columns WHERE table_name = 'users' LIMIT 10;

-- ============================================================================
-- 9. Performance Schema Queries
-- ============================================================================

-- VULNERABLE: Query performance schema
-- ruleid: information-disclosure-009
SELECT * FROM performance_schema.events_statements_history;

-- VULNERABLE: Query table I/O
-- ruleid: information-disclosure-009
SELECT * FROM performance_schema.table_io_waits_summary_by_table;

-- VULNERABLE: Query user statistics
-- ruleid: information-disclosure-009
SELECT * FROM performance_schema.users;

-- SAFE: Limited performance monitoring
SELECT COUNT(*) FROM performance_schema.events_statements_history;

-- ============================================================================
-- 10. Stored Procedure Information
-- ============================================================================

-- VULNERABLE: Query procedure source
-- ruleid: information-disclosure-010
SELECT * FROM information_schema.routines;

-- VULNERABLE: Query procedure parameters
-- ruleid: information-disclosure-010
SELECT * FROM information_schema.parameters;

-- VULNERABLE: Query triggers
-- ruleid: information-disclosure-010
SELECT * FROM information_schema.triggers;

-- SAFE: Query only procedure names
SELECT routine_name FROM information_schema.routines WHERE routine_schema = 'myapp';

-- ============================================================================
-- 11. View Information
-- ============================================================================

-- VULNERABLE: Query view definitions
-- ruleid: information-disclosure-011
SELECT * FROM information_schema.views;

-- VULNERABLE: Query view source
-- ruleid: information-disclosure-011
SELECT view_definition FROM information_schema.views;

-- SAFE: Query only view names
SELECT table_name FROM information_schema.views WHERE table_schema = 'myapp';

-- ============================================================================
-- 12. Privilege Information Disclosure
-- ============================================================================

-- VULNERABLE: Query all user privileges
-- ruleid: information-disclosure-012
SELECT * FROM information_schema.user_privileges;

-- VULNERABLE: Query table privileges
-- ruleid: information-disclosure-012
SELECT * FROM information_schema.table_privileges;

-- VULNERABLE: Query column privileges
-- ruleid: information-disclosure-012
SELECT * FROM information_schema.column_privileges;

-- VULNERABLE: Query schema privileges
-- ruleid: information-disclosure-012
SELECT * FROM information_schema.schema_privileges;

-- SAFE: Query current user privileges
SHOW GRANTS FOR CURRENT_USER();

-- ============================================================================
-- 13. Configuration Information
-- ============================================================================

-- VULNERABLE: Query configuration variables
-- ruleid: information-disclosure-013
SELECT @@version;

-- VULNERABLE: Query data directory
-- ruleid: information-disclosure-013
SELECT @@datadir;

-- VULNERABLE: Query base directory
-- ruleid: information-disclosure-013
SELECT @@basedir;

-- VULNERABLE: Query plugin directory
-- ruleid: information-disclosure-013
SELECT @@plugin_dir;

-- SAFE: Application-controlled configuration
SELECT 'production' as environment;

-- ============================================================================
-- 14. Session Information
-- ============================================================================

-- VULNERABLE: Query session variables
-- ruleid: information-disclosure-014
SELECT @@session.sql_mode;

-- VULNERABLE: Query connection information
-- ruleid: information-disclosure-014
SELECT CONNECTION_ID();

-- VULNERABLE: Query thread information
-- ruleid: information-disclosure-014
SELECT * FROM information_schema.processlist;

-- SAFE: Query only necessary session info
SELECT @@session.time_zone;

