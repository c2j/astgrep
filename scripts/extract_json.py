import sys
import re
import json

def extract_json_from_output(output):
    # Find JSON object starting with {"findings":
    match = re.search(r'\{"findings":\s*\[[\s\S]*?\},\s*"summary":\s*\{[\s\S]*?\}', output)
    if match:
        return match.group(0)
    else:
        return '{"findings":[]}'

if __name__ == "__main__":
    with open('/Users/c2j/.local/share/opencode/tool-output/tool_c7dcabf99001LjDTOEgCRYb2fy', 'r') as f:
        output = f.read()
    json_str = extract_json_from_output(output)
    try:
        data = json.loads(json_str)
        print(json.dumps(data, indent=2))
        print(f"Number of findings: {len(data['findings'])}")
    except Exception as e:
        print(f"Error parsing JSON: {e}")
        print(f"Extracted JSON: {repr(json_str)}")
