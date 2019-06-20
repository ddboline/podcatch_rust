#!/usr/bin/env python3

import os
import json
from google_music import MusicManager

def google_music_api():
    mm = MusicManager('ddboline')

    uploaded = mm.songs(uploaded=True, purchased=False)

    with open('uploaded_mp3.jsonl', 'w') as f:
        for item in uploaded:
            f.write(json.dumps(item))
            f.write('\n')


def upload_file(fname):
    mm = MusicManager('ddboline')

    for line in open(fname):
        line = line.strip()
        print(f'upload {line}')
        mm.upload(line)


if __name__ == '__main__':
    if len(os.sys.argv) == 1:
        google_music_api()
    else:
        for arg in os.sys.argv[1:]:
            if os.path.exists(arg):
                upload_file(arg)
