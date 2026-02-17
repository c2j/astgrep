# Test file for pattern-not-regex functionality

# Import tests
import foo  # Should match - simple foo import
import foo_bar  # Should NOT match - compound with underscore
import foo-bar  # Should NOT match - compound with dash
import bar_foo  # Should NOT match - compound ending with foo
import bar  # Should NOT match - different package
from foo import something  # Should match - from foo import

# Variable assignment tests
x = 42  # Should match - simple variable
name = "test"  # Should match - simple variable
user_id = 123  # Should match - simple variable with underscore
data = [1, 2, 3]  # Should match - simple variable

# These should NOT match (excluded by pattern-not-regex)
arr[0] = value  # Should NOT match - array access
obj.property = value  # Should NOT match - attribute access
user.name = "John"  # Should NOT match - attribute access
items[index] = new_value  # Should NOT match - array access

# URL tests
url1 = "http://example.com"  # Should match - HTTP URL
url2 = "https://secure.com"  # Should NOT match - HTTPS URL
api_endpoint = "http://api.service.com/data"  # Should match - HTTP URL
secure_api = "https://api.secure.com/data"  # Should NOT match - HTTPS URL

# Mixed content
config = {
    "insecure": "http://legacy.system.com",  # Should match
    "secure": "https://new.system.com",      # Should NOT match
    "local": "http://localhost:8080"         # Should match
}
