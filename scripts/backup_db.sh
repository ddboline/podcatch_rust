#!/bin/bash

DB="podcatch"
BUCKET="podcatch-db-backup"

TABLES="
episodes
podcasts
google_music_metadata
"

mkdir -p backup

for T in $TABLES;
do
    psql $DB -c "COPY $T TO STDOUT" | gzip > backup/${T}.sql.gz
    aws s3 cp backup/${T}.sql.gz s3://${BUCKET}/${T}.sql.gz
done
