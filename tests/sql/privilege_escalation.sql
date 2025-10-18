-- Privilege Escalation Test Cases
-- This file contains examples of excessive privilege grants and security issues

-- ============================================================================
-- 1. GRANT ALL PRIVILEGES
-- ============================================================================

-- VULNERABLE: GRANT ALL PRIVILEGES on all databases
-- ruleid: privilege-escalation-001
GRANT ALL PRIVILEGES ON *.* TO 'webapp'@'%';

-- VULNERABLE: GRANT ALL PRIVILEGES with GRANT OPTION
-- ruleid: privilege-escalation-001
GRANT ALL PRIVILEGES ON *.* TO 'service'@'localhost' WITH GRANT OPTION;

-- VULNERABLE: GRANT ALL on specific database
-- ruleid: privilege-escalation-001
GRANT ALL ON myapp.* TO 'webapp'@'%';

-- VULNERABLE: GRANT ALL with wildcard host
-- ruleid: privilege-escalation-001
GRANT ALL PRIVILEGES ON *.* TO 'admin'@'%';

-- SAFE: GRANT specific privileges
GRANT SELECT, INSERT ON myapp.* TO 'webapp'@'%';

-- SAFE: GRANT limited privileges
GRANT SELECT ON myapp.users TO 'readonly'@'localhost';

-- SAFE: GRANT with specific host
GRANT SELECT, INSERT, UPDATE ON myapp.* TO 'webapp'@'192.168.1.0/24';

-- ============================================================================
-- 2. GRANT ALL ON *.*
-- ============================================================================

-- VULNERABLE: GRANT ALL ON *.*
-- ruleid: privilege-escalation-002
GRANT ALL ON *.* TO 'user'@'localhost';

-- VULNERABLE: GRANT ALL ON *.* to multiple users
-- ruleid: privilege-escalation-002
GRANT ALL ON *.* TO 'user1'@'%', 'user2'@'%';

-- VULNERABLE: GRANT ALL ON *.* with IDENTIFIED BY
-- ruleid: privilege-escalation-002
GRANT ALL ON *.* TO 'newuser'@'%' IDENTIFIED BY 'password';

-- SAFE: GRANT specific privileges on specific database
GRANT SELECT, INSERT, UPDATE ON myapp.* TO 'user'@'localhost';

-- SAFE: GRANT with limited scope
GRANT SELECT ON myapp.users TO 'user'@'localhost';

-- ============================================================================
-- 3. ALTER USER with Privilege Changes
-- ============================================================================

-- VULNERABLE: ALTER USER with excessive privileges
-- ruleid: privilege-escalation-003
ALTER USER 'admin'@'%' IDENTIFIED BY 'newpassword';

-- VULNERABLE: ALTER USER to grant DBA role
-- ruleid: privilege-escalation-003
ALTER USER 'user'@'localhost' GRANT ROLE 'dba';

-- VULNERABLE: ALTER USER with SUPER privilege
-- ruleid: privilege-escalation-003
ALTER USER 'service'@'%' GRANT ROLE 'super_admin';

-- SAFE: ALTER USER password only
ALTER USER 'user'@'localhost' IDENTIFIED BY 'newpassword';

-- SAFE: ALTER USER with limited role
ALTER USER 'user'@'localhost' GRANT ROLE 'read_only';

-- ============================================================================
-- 4. CREATE USER with Excessive Privileges
-- ============================================================================

-- VULNERABLE: CREATE USER with SUPER privilege
-- ruleid: privilege-escalation-004
CREATE USER 'admin'@'%' IDENTIFIED BY 'password' WITH GRANT OPTION;

-- VULNERABLE: CREATE USER with all privileges
-- ruleid: privilege-escalation-004
CREATE USER 'service'@'localhost' IDENTIFIED BY 'password';
GRANT ALL ON *.* TO 'service'@'localhost';

-- SAFE: CREATE USER with limited privileges
CREATE USER 'webapp'@'localhost' IDENTIFIED BY 'password';
GRANT SELECT, INSERT, UPDATE ON myapp.* TO 'webapp'@'localhost';

-- SAFE: CREATE USER with read-only access
CREATE USER 'readonly'@'localhost' IDENTIFIED BY 'password';
GRANT SELECT ON myapp.* TO 'readonly'@'localhost';

-- ============================================================================
-- 5. GRANT with GRANT OPTION
-- ============================================================================

-- VULNERABLE: GRANT with GRANT OPTION
-- ruleid: privilege-escalation-005
GRANT ALL PRIVILEGES ON *.* TO 'user'@'%' WITH GRANT OPTION;

-- VULNERABLE: GRANT SELECT with GRANT OPTION
-- ruleid: privilege-escalation-005
GRANT SELECT ON *.* TO 'user'@'%' WITH GRANT OPTION;

-- VULNERABLE: GRANT with GRANT OPTION on all databases
-- ruleid: privilege-escalation-005
GRANT ALL ON *.* TO 'admin'@'localhost' WITH GRANT OPTION;

-- SAFE: GRANT without GRANT OPTION
GRANT SELECT, INSERT ON myapp.* TO 'user'@'%';

-- SAFE: GRANT limited privileges without GRANT OPTION
GRANT SELECT ON myapp.users TO 'user'@'localhost';

-- ============================================================================
-- 6. GRANT Administrative Privileges
-- ============================================================================

-- VULNERABLE: GRANT SUPER privilege
-- ruleid: privilege-escalation-006
GRANT SUPER ON *.* TO 'user'@'%';

-- VULNERABLE: GRANT FILE privilege
-- ruleid: privilege-escalation-006
GRANT FILE ON *.* TO 'user'@'%';

-- VULNERABLE: GRANT PROCESS privilege
-- ruleid: privilege-escalation-006
GRANT PROCESS ON *.* TO 'user'@'%';

-- VULNERABLE: GRANT RELOAD privilege
-- ruleid: privilege-escalation-006
GRANT RELOAD ON *.* TO 'user'@'%';

-- SAFE: GRANT application-specific privileges
GRANT SELECT, INSERT, UPDATE ON myapp.* TO 'user'@'%';

-- SAFE: GRANT limited administrative privileges
GRANT CREATE, ALTER ON myapp.* TO 'admin'@'localhost';

-- ============================================================================
-- 7. GRANT to Wildcard Hosts
-- ============================================================================

-- VULNERABLE: GRANT to wildcard host %
-- ruleid: privilege-escalation-007
GRANT ALL ON *.* TO 'user'@'%';

-- VULNERABLE: GRANT to broad IP range
-- ruleid: privilege-escalation-007
GRANT ALL ON *.* TO 'user'@'192.168.%';

-- VULNERABLE: GRANT to any host
-- ruleid: privilege-escalation-007
GRANT SELECT ON *.* TO 'user'@'%';

-- SAFE: GRANT to specific host
GRANT SELECT ON myapp.* TO 'user'@'192.168.1.100';

-- SAFE: GRANT to localhost only
GRANT SELECT ON myapp.* TO 'user'@'localhost';

-- SAFE: GRANT to specific subnet
GRANT SELECT ON myapp.* TO 'user'@'192.168.1.0/24';

-- ============================================================================
-- 8. REVOKE Insufficient Privileges
-- ============================================================================

-- VULNERABLE: REVOKE but user still has excessive privileges
-- ruleid: privilege-escalation-008
GRANT ALL ON *.* TO 'user'@'%';
REVOKE INSERT ON *.* FROM 'user'@'%';

-- VULNERABLE: Incomplete privilege revocation
-- ruleid: privilege-escalation-008
GRANT ALL ON *.* TO 'user'@'%';
REVOKE SELECT ON myapp.* FROM 'user'@'%';

-- SAFE: Complete privilege revocation
GRANT SELECT ON myapp.* TO 'user'@'%';
REVOKE ALL ON *.* FROM 'user'@'%';

-- SAFE: Proper privilege management
GRANT SELECT, INSERT ON myapp.users TO 'user'@'%';
REVOKE INSERT ON myapp.users FROM 'user'@'%';

-- ============================================================================
-- 9. Default User Privileges
-- ============================================================================

-- VULNERABLE: Default user with excessive privileges
-- ruleid: privilege-escalation-009
GRANT ALL ON *.* TO 'root'@'%';

-- VULNERABLE: Anonymous user with privileges
-- ruleid: privilege-escalation-009
GRANT SELECT ON *.* TO ''@'%';

-- SAFE: Root limited to localhost
GRANT ALL ON *.* TO 'root'@'localhost';

-- SAFE: Anonymous user removed
DROP USER ''@'%';

-- ============================================================================
-- 10. Role-based Privilege Escalation
-- ============================================================================

-- VULNERABLE: Grant admin role to application user
-- ruleid: privilege-escalation-010
GRANT 'admin_role' TO 'webapp'@'%';

-- VULNERABLE: Grant DBA role to service account
-- ruleid: privilege-escalation-010
GRANT 'dba_role' TO 'service'@'localhost';

-- SAFE: Grant application-specific role
GRANT 'app_user_role' TO 'webapp'@'%';

-- SAFE: Grant read-only role
GRANT 'read_only_role' TO 'readonly'@'localhost';

-- ============================================================================
-- 11. Privilege Escalation in Stored Procedures
-- ============================================================================

-- VULNERABLE: Stored procedure with DEFINER privilege escalation
-- ruleid: privilege-escalation-011
CREATE DEFINER='admin'@'%' PROCEDURE GetAllUsers()
BEGIN
    SELECT * FROM users;
END;

-- VULNERABLE: Stored procedure with excessive privileges
-- ruleid: privilege-escalation-011
CREATE DEFINER='root'@'localhost' PROCEDURE DeleteAllData()
BEGIN
    DELETE FROM users;
END;

-- SAFE: Stored procedure with limited DEFINER
CREATE DEFINER='app_user'@'localhost' PROCEDURE GetUserData(IN user_id INT)
BEGIN
    SELECT * FROM users WHERE id = user_id;
END;

-- ============================================================================
-- 12. Privilege Escalation in Views
-- ============================================================================

-- VULNERABLE: View with DEFINER privilege escalation
-- ruleid: privilege-escalation-012
CREATE DEFINER='admin'@'%' VIEW admin_view AS
SELECT * FROM sensitive_data;

-- VULNERABLE: View with excessive privileges
-- ruleid: privilege-escalation-012
CREATE DEFINER='root'@'localhost' VIEW all_users_view AS
SELECT * FROM users;

-- SAFE: View with limited DEFINER
CREATE DEFINER='app_user'@'localhost' VIEW user_profile_view AS
SELECT id, username, email FROM users;

