#!/bin/bash

DB="podcatch"

TABLES="
episodes
podcasts
google_music_metadata
"

for T in $TABLES;
do
    psql $DB -c "DELETE FROM $T";
done
