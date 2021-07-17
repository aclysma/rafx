import pathlib
import os
import logging
logging = logging.getLogger(__name__)

def write_bytes_to_file(path, bytes):
    logging.info("Writing %d bytes to file %s", len(bytes), path)
    dir = os.path.dirname(path)
    pathlib.Path(dir).mkdir(parents=True, exist_ok=True)
    with open(path, "wb") as f:
        f.write(bytes)

def write_string_to_file(path, s):
    logging.info("Writing %d chars to file %s", len(s), path)
    dir = os.path.dirname(path)
    pathlib.Path(dir).mkdir(parents=True, exist_ok=True)
    with open(path, "w") as f:
        f.write(s)

