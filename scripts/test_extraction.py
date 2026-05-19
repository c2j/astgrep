import re
import json

with open('astgrep_debug_output.txt', 'r') as f:
    output = f.read()

print("Output:", repr(output))
# Find JSON object starting with {"findings": and containing "summary":
match = re.search(r'\{"findings":\s*\[[\s\S]*?"summary":\s*\{[\s\S]*?\}', output)
if match:
    print("Match found:", repr(match.group(0)))
    try:
        data = json.loads(match.group(0))
        print("Number of findings:", len(data['findings']))
    except Exception as e:
        print("Error parsing JSON:", e)
else:
    print("No match found")
