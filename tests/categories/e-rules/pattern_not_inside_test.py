# Test file for pattern-not-inside functionality

# Should match - function outside class
def standalone_function():
    return "test"

# Should match - another standalone function
def another_function(param):
    print(param)

class MyClass:
    # Should NOT match - function inside class
    def method_inside_class(self):
        return "test"
    
    def another_method(self):
        pass

# Should match - function outside class again
def global_function():
    pass

# Test eval() outside sandbox
def regular_function():
    # Should match - eval outside sandbox
    eval("print('dangerous')")
    
def sandbox():
    # Should NOT match - eval inside sandbox
    eval("print('safe')")

def another_sandbox(code):
    # Should NOT match - eval inside sandbox
    eval(code)

# Should match - eval outside sandbox
eval("malicious_code")

# Test SQL queries
import sqlite3

def unsafe_query():
    cursor = sqlite3.cursor()
    # Should match - query outside transaction
    cursor.execute("SELECT * FROM users")

def safe_query():
    connection = sqlite3.connect("db.sqlite")
    with connection.begin():
        cursor = connection.cursor()
        # Should NOT match - query inside transaction
        cursor.execute("SELECT * FROM users")

# Should match - query outside transaction
cursor = sqlite3.cursor()
cursor.execute("DELETE FROM users WHERE id = 1")
