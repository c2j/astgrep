-- Weak Encryption Test Cases
-- This file contains examples of weak cryptographic algorithms in SQL

-- ============================================================================
-- 1. MD5 Hashing
-- ============================================================================

-- VULNERABLE: MD5 hashing for passwords
-- ruleid: weak-encryption-001
SELECT MD5(password) FROM users;

-- VULNERABLE: MD5 in INSERT
-- ruleid: weak-encryption-001
INSERT INTO users (username, password_hash) VALUES ('admin', MD5('password123'));

-- VULNERABLE: MD5 in UPDATE
-- ruleid: weak-encryption-001
UPDATE users SET password_hash = MD5('newpassword') WHERE id = 1;

-- VULNERABLE: MD5 in CREATE TABLE
-- ruleid: weak-encryption-001
CREATE TABLE user_hashes (
    id INT,
    password_hash VARCHAR(32) DEFAULT MD5('default')
);

-- VULNERABLE: MD5 in VIEW
-- ruleid: weak-encryption-001
CREATE VIEW user_passwords AS
SELECT id, username, MD5(password) as password_hash FROM users;

-- VULNERABLE: MD5 in INDEX
-- ruleid: weak-encryption-001
CREATE INDEX idx_password_hash ON users(MD5(password));

-- SAFE: SHA256 hashing
SELECT SHA256(password) FROM users;

-- SAFE: BCRYPT hashing
SELECT BCRYPT(password) FROM users;

-- ============================================================================
-- 2. SHA1 Hashing
-- ============================================================================

-- VULNERABLE: SHA1 hashing
-- ruleid: weak-encryption-002
SELECT SHA1(password) FROM users;

-- VULNERABLE: SHA1 in INSERT
-- ruleid: weak-encryption-002
INSERT INTO users (username, password_hash) VALUES ('user', SHA1('password'));

-- VULNERABLE: SHA1 in UPDATE
-- ruleid: weak-encryption-002
UPDATE users SET password_hash = SHA1('newpassword') WHERE id = 1;

-- VULNERABLE: SHA1 in stored procedure
-- ruleid: weak-encryption-002
CREATE PROCEDURE HashPassword(IN pwd VARCHAR(255))
BEGIN
    SELECT SHA1(pwd);
END;

-- SAFE: SHA256 instead of SHA1
SELECT SHA256(password) FROM users;

-- SAFE: SCRYPT hashing
SELECT SCRYPT(password) FROM users;

-- ============================================================================
-- 3. DES Encryption
-- ============================================================================

-- VULNERABLE: DES_ENCRYPT
-- ruleid: weak-encryption-003
SELECT DES_ENCRYPT(credit_card, 'key') FROM payments;

-- VULNERABLE: DES_ENCRYPT in INSERT
-- ruleid: weak-encryption-003
INSERT INTO payments (card_number) VALUES (DES_ENCRYPT('4111111111111111', 'secret'));

-- VULNERABLE: DES_ENCRYPT in UPDATE
-- ruleid: weak-encryption-003
UPDATE payments SET card_number = DES_ENCRYPT('4111111111111111', 'key') WHERE id = 1;

-- VULNERABLE: DES_DECRYPT
-- ruleid: weak-encryption-003
SELECT DES_DECRYPT(card_number, 'key') FROM payments;

-- SAFE: AES_ENCRYPT
SELECT AES_ENCRYPT(credit_card, 'key') FROM payments;

-- SAFE: Modern encryption
SELECT ENCRYPT(credit_card, 'key') FROM payments;

-- ============================================================================
-- 4. Weak Hashing in Triggers
-- ============================================================================

-- VULNERABLE: MD5 in trigger
-- ruleid: weak-encryption-004
CREATE TRIGGER hash_password_on_insert
BEFORE INSERT ON users
FOR EACH ROW
BEGIN
    SET NEW.password_hash = MD5(NEW.password);
END;

-- VULNERABLE: SHA1 in trigger
-- ruleid: weak-encryption-004
CREATE TRIGGER hash_password_on_update
BEFORE UPDATE ON users
FOR EACH ROW
BEGIN
    SET NEW.password_hash = SHA1(NEW.password);
END;

-- SAFE: SHA256 in trigger
CREATE TRIGGER hash_password_on_insert
BEFORE INSERT ON users
FOR EACH ROW
BEGIN
    SET NEW.password_hash = SHA256(NEW.password);
END;

-- ============================================================================
-- 5. Weak Encryption in Stored Procedures
-- ============================================================================

-- VULNERABLE: MD5 in stored procedure
-- ruleid: weak-encryption-005
CREATE PROCEDURE CreateUser(IN username VARCHAR(255), IN password VARCHAR(255))
BEGIN
    INSERT INTO users (username, password_hash) VALUES (username, MD5(password));
END;

-- VULNERABLE: SHA1 in stored procedure
-- ruleid: weak-encryption-005
CREATE PROCEDURE UpdatePassword(IN user_id INT, IN new_password VARCHAR(255))
BEGIN
    UPDATE users SET password_hash = SHA1(new_password) WHERE id = user_id;
END;

-- VULNERABLE: DES_ENCRYPT in stored procedure
-- ruleid: weak-encryption-005
CREATE PROCEDURE EncryptCard(IN card_number VARCHAR(255))
BEGIN
    SELECT DES_ENCRYPT(card_number, 'key');
END;

-- SAFE: SHA256 in stored procedure
CREATE PROCEDURE CreateUser(IN username VARCHAR(255), IN password VARCHAR(255))
BEGIN
    INSERT INTO users (username, password_hash) VALUES (username, SHA256(password));
END;

-- ============================================================================
-- 6. Weak Encryption in Views
-- ============================================================================

-- VULNERABLE: MD5 in view
-- ruleid: weak-encryption-006
CREATE VIEW user_data AS
SELECT id, username, MD5(password) as password_hash FROM users;

-- VULNERABLE: SHA1 in view
-- ruleid: weak-encryption-006
CREATE VIEW sensitive_data AS
SELECT id, email, SHA1(phone) as phone_hash FROM users;

-- VULNERABLE: DES_ENCRYPT in view
-- ruleid: weak-encryption-006
CREATE VIEW payment_data AS
SELECT id, DES_ENCRYPT(card_number, 'key') as encrypted_card FROM payments;

-- SAFE: SHA256 in view
CREATE VIEW user_data AS
SELECT id, username, SHA256(password) as password_hash FROM users;

-- ============================================================================
-- 7. Multiple Weak Algorithms
-- ============================================================================

-- VULNERABLE: Multiple weak hashes
-- ruleid: weak-encryption-007
SELECT 
    MD5(password) as md5_hash,
    SHA1(password) as sha1_hash,
    DES_ENCRYPT(ssn, 'key') as encrypted_ssn
FROM users;

-- VULNERABLE: Weak encryption in complex query
-- ruleid: weak-encryption-007
SELECT 
    u.id,
    MD5(u.password) as password_hash,
    SHA1(u.email) as email_hash,
    DES_ENCRYPT(p.card_number, 'key') as card
FROM users u
JOIN payments p ON u.id = p.user_id;

-- SAFE: Strong encryption
SELECT 
    SHA256(password) as password_hash,
    SHA256(email) as email_hash,
    AES_ENCRYPT(ssn, 'key') as encrypted_ssn
FROM users;

-- ============================================================================
-- 8. Weak Encryption in Indexes
-- ============================================================================

-- VULNERABLE: MD5 in index
-- ruleid: weak-encryption-008
CREATE INDEX idx_password ON users(MD5(password));

-- VULNERABLE: SHA1 in index
-- ruleid: weak-encryption-008
CREATE INDEX idx_email ON users(SHA1(email));

-- VULNERABLE: DES_ENCRYPT in index
-- ruleid: weak-encryption-008
CREATE INDEX idx_card ON payments(DES_ENCRYPT(card_number, 'key'));

-- SAFE: SHA256 in index
CREATE INDEX idx_password ON users(SHA256(password));

-- ============================================================================
-- 9. Weak Encryption in Constraints
-- ============================================================================

-- VULNERABLE: MD5 in CHECK constraint
-- ruleid: weak-encryption-009
CREATE TABLE users (
    id INT,
    password VARCHAR(255),
    CHECK (MD5(password) != '')
);

-- VULNERABLE: SHA1 in DEFAULT
-- ruleid: weak-encryption-009
CREATE TABLE users (
    id INT,
    password_hash VARCHAR(40) DEFAULT SHA1('default')
);

-- SAFE: Strong encryption in DEFAULT
CREATE TABLE users (
    id INT,
    password_hash VARCHAR(64) DEFAULT SHA256('default')
);

-- ============================================================================
-- 10. Weak Encryption in Transactions
-- ============================================================================

-- VULNERABLE: MD5 in transaction
-- ruleid: weak-encryption-010
START TRANSACTION;
INSERT INTO users (username, password_hash) VALUES ('user', MD5('password'));
COMMIT;

-- VULNERABLE: SHA1 in transaction
-- ruleid: weak-encryption-010
BEGIN;
UPDATE users SET password_hash = SHA1('newpassword') WHERE id = 1;
COMMIT;

-- SAFE: SHA256 in transaction
START TRANSACTION;
INSERT INTO users (username, password_hash) VALUES ('user', SHA256('password'));
COMMIT;

-- ============================================================================
-- 11. Legacy Weak Encryption
-- ============================================================================

-- VULNERABLE: OLD_PASSWORD (MySQL legacy)
-- ruleid: weak-encryption-011
SELECT OLD_PASSWORD(password) FROM users;

-- VULNERABLE: PASSWORD function (deprecated)
-- ruleid: weak-encryption-011
UPDATE users SET password_hash = PASSWORD('newpassword');

-- SAFE: Modern password hashing
UPDATE users SET password_hash = SHA256(CONCAT(password, 'salt'));

-- ============================================================================
-- 12. Weak Encryption with Hardcoded Keys
-- ============================================================================

-- VULNERABLE: DES with hardcoded key
-- ruleid: weak-encryption-012
SELECT DES_ENCRYPT(credit_card, 'hardcoded_key') FROM payments;

-- VULNERABLE: MD5 with hardcoded salt
-- ruleid: weak-encryption-012
SELECT MD5(CONCAT(password, 'fixed_salt')) FROM users;

-- VULNERABLE: SHA1 with hardcoded salt
-- ruleid: weak-encryption-012
UPDATE users SET password_hash = SHA1(CONCAT(password, 'my_salt'));

-- SAFE: Use proper key management
SELECT AES_ENCRYPT(credit_card, @encryption_key) FROM payments;

