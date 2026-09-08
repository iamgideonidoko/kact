#!/usr/bin/env python3
"""Create a reproducible gzip-compressed tar archive from a staged directory."""

import gzip
import os
import sys
import tarfile
from pathlib import Path

source = Path(sys.argv[1])
archive = Path(sys.argv[2])
timestamp = int(os.environ.get("SOURCE_DATE_EPOCH", "0"))

with archive.open("wb") as output, gzip.GzipFile(
    filename="", mode="wb", fileobj=output, mtime=timestamp
) as compressed, tarfile.open(fileobj=compressed, mode="w") as tar:
    for path in [source, *sorted(source.rglob("*"))]:
        info = tar.gettarinfo(str(path), arcname=str(path.relative_to(source.parent)))
        info.uid = info.gid = 0
        info.uname = info.gname = ""
        info.mtime = timestamp
        info.mode = 0o755 if info.isdir() or info.mode & 0o111 else 0o644
        if info.isfile():
            with path.open("rb") as contents:
                tar.addfile(info, contents)
        else:
            tar.addfile(info)
