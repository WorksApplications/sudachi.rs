#   Copyright (c) 2021-2024 Works Applications Co., Ltd.
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

# Set the Python package version in pyproject.toml to the next unused version.
# This script is used to upload to TestPyPI (that does not allow same version) in python-upload-test workflow.
#
# 1. if current version has pre/post/dev part, increment the last part
# 2. if current version is final version, add post part
#
# we should avoid `.devN` if possible, since it's hard to handle version order with it.
#   e.g. `1.2.dev1` < `1.2a1.dev1` < `1.2a1` < `1.2`
# ref: https://packaging.python.org/en/latest/specifications/version-specifiers/

import json
import sys
import urllib.request
from pathlib import Path
from packaging.version import Version, InvalidVersion
import tomlkit

# find current version
cur_file = Path(__file__)
pyproject = cur_file.parent.parent / "pyproject.toml"

with pyproject.open("rt", encoding="utf-8") as f:
    pyproject_data = f.read()

pyproject_doc = tomlkit.parse(pyproject_data)
cur_version = pyproject_doc.get("project", {}).get("version")

if not cur_version:
    print("could not find version", file=sys.stderr)
    exit(1)

try:
    cur_version = Version(cur_version)
    print("Current version:", cur_version)
except InvalidVersion:
    print(f"{cur_version} is invalid as a python version")
    exit(1)

# find remote versions (in TestPyPI)
response = urllib.request.urlopen("https://test.pypi.org/pypi/SudachiPy/json")
data = json.loads(response.read())

remote_versions = set(data["releases"].keys())

# add deleted version to the list
remote_versions.add("0.6.0")

print("Remote versions:", sorted(remote_versions))


def increment_version(v: Version):
    pre = v.pre
    post = v.post
    dev = v.dev

    if v.is_devrelease:
        dev += 1
    elif v.is_postrelease:
        post += 1
    elif v.is_prerelease:
        pre = (pre[0], pre[1] + 1)
    else:  # is final release
        post = 1

    next_version = v.base_version + \
        ("" if pre is None else f"{pre[0]}{pre[1]}") + \
        ("" if post is None else f".post{post}") + \
        ("" if dev is None else f".dev{dev}")

    assert Version(next_version) > v
    return Version(next_version)


# search proper version to upload
next_v = cur_version

while str(next_v) in remote_versions:
    next_v = increment_version(next_v)


print("::notice::Next version:", next_v)

pyproject_doc["project"]["version"] = str(next_v)

with pyproject.open("wt", encoding='utf-8') as f:
    f.write(tomlkit.dumps(pyproject_doc))
