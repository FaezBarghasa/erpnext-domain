import os
import json
import re

brain_dir = "/home/jrad/.gemini/antigravity-ide/brain/"
results = []

for root, dirs, files in os.walk(brain_dir):
    if "transcript.jsonl" in files:
        path = os.path.join(root, "transcript.jsonl")
        try:
            with open(path) as f:
                for idx, line in enumerate(f):
                    obj = json.loads(line)
                    content = obj.get("content", "")
                    if "Phase 7" in content:
                        results.append(f"=== PATH: {path} (Step {idx}) ===\n")
                        for m in re.finditer("Phase 7", content, re.IGNORECASE):
                            start = max(0, m.start() - 200)
                            end = min(len(content), m.start() + 1000)
                            results.append(content[start:end])
                            results.append("\n" + "-"*60 + "\n")
        except Exception as e:
            pass

with open("/home/jrad/RustroverProjects/ERPNext_workspace/erpnext-domain/found_phase7_transcripts.txt", "w") as out:
    out.write("\n".join(results))
print(f"Done! Written {len(results)} lines/blocks to found_phase7_transcripts.txt.")
