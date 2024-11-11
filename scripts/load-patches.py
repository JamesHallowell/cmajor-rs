#!/usr/bin/env python3

from urllib.request import urlopen
from tempfile import NamedTemporaryFile
from subprocess import call

base_url = "https://raw.githubusercontent.com/cmajor-lang/cmajor/refs/heads/main/examples/patches"

examples = [
    "SineSynth/SineSynth.cmajor"
]

for example in examples:
    print(f"Compiling {example}...")

    url = f"{base_url}/{example}"
    with urlopen(url) as response:
        code = response.read()
        with NamedTemporaryFile(mode="w", suffix=".cmajor") as cmajor_file:
            cmajor_file.write(code.decode())
            cmajor_file.flush()

            result = call(["cargo", "run", "--example", "patch", "--", cmajor_file.name])
            if result != 0:
                raise Exception(f"Failed to compile {example}")
