#!/usr/bin/env python3

from io import BytesIO
import os
import shutil
from subprocess import call
from tempfile import TemporaryDirectory, NamedTemporaryFile
from urllib.request import urlopen
from zipfile import ZipFile


def download_github_release(asset):
    url = f"https://github.com/SoundStacks/cmajor/releases/download/{asset}"
    response = urlopen(url)
    if response.status != 200:
        raise Exception(f"Failed to download {url}")
    body = response.read()
    return body


def fetch_linux(asset, destination):
    asset = download_github_release(asset)
    with TemporaryDirectory() as tmpdir:
        with ZipFile(BytesIO(asset)) as archive:
            archive.extractall(tmpdir)
            for root, dirs, files in os.walk(tmpdir):
                for file in files:
                    if file == "libCmajPerformer.so":
                        file_path = os.path.join(root, file)
                        if not os.path.exists(destination):
                            os.makedirs(destination)
                        shutil.move(file_path, destination)
                        return


def fetch_macos(asset, destination):
    asset = download_github_release(asset)
    with NamedTemporaryFile(suffix='.dmg') as temp_file:
        temp_file.write(asset)
        temp_file.flush()

        call(["hdiutil", "attach", temp_file.name, "-mountpoint", "/Volumes/cmajor"])

        for root, dirs, files in os.walk("/Volumes/cmajor"):
            for file in files:
                if file == "libCmajPerformer.dylib":
                    file_path = os.path.join(root, file)
                    if not os.path.exists(destination):
                        os.makedirs(destination)
                    shutil.copy(file_path, destination)

        call(["hdiutil", "detach", "/Volumes/cmajor"])


def fetch_cmajor(version, platform, arch):
    if platform == "linux":
        if arch is None:
            raise Exception("Arch must be specified for Linux")
        fetch_linux(f"{version}/cmajor.linux.{arch}.zip", f"cmaj/linux/{arch}")
    elif platform == "macos":
        fetch_macos(f"{version}/cmajor.dmg", "cmaj/macos")
    else:
        raise Exception(f"Unsupported platform {platform} {arch}")


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("version", help="The version of cmajor to fetch")
    parser.add_argument("platform", help="The platform to fetch")
    parser.add_argument("--arch", help="An optional arch", required=False)
    args = parser.parse_args()

    fetch_cmajor(args.version, args.platform, args.arch)
