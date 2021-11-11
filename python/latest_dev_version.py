#   Copyright (c) 2021 Works Applications Co., Ltd.
#
#   Licensed under the Apache License, Version 2.0 (the "License");
#   you may not use this file except in compliance with the License.
#   You may obtain a copy of the License at
#
#       http://www.apache.org/licenses/LICENSE-2.0
#
#    Unless required by applicable law or agreed to in writing, software
#   distributed under the License is distributed on an "AS IS" BASIS,
#   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#   See the License for the specific language governing permissions and
#   limitations under the License.

# Set the version in setup.py to the next unused .dev version
# Used versions are acquired directly from

import json
import re
import sys
import urllib.request
from pathlib import Path

cur_file = Path(__file__)

setup_py = cur_file.parent / "setup.py"

with setup_py.open("rt", encoding="utf-8") as f:
    setup_py_data = f.read()

version_re = re.compile('version="([^"]+)",')
cur_version = version_re.findall(setup_py_data)

if len(cur_version) != 1:
    print("could not find version", sys.stderr)
    exit(1)

cur_version = cur_version[0]

print("Current version:", cur_version)

if "dev" in cur_version:
    print("Can't modify dev version")
    exit(1)

response = urllib.request.urlopen("https://test.pypi.org/pypi/SudachiPy/json")
data = json.loads(response.read())

remote_versions = set(data["releases"].keys())

remote_versions.add("0.6.0")  # it was deleted

next_version_re = re.compile("""^(.*)\.dev(\d+)$""")


def next_version(version):
    m = next_version_re.match(version)
    if m is None:
        return version + ".dev1"
    else:
        p1 = m.group(1)
        p2 = int(m.group(2))
        return "{}.dev{}".format(p1, p2 + 1)


print("Remote versions:", sorted(remote_versions))

next_v = next_version(cur_version)

while next_v in remote_versions:
    next_v = next_version(next_v)

print("::notice::Next version:", next_v)

modified_setup_py = version_re.sub('version="{}",'.format(next_v), setup_py_data, 1)

with setup_py.open("wt", encoding='utf-8') as f:
    f.write(modified_setup_py)
