#!/bin/bash

DB="podcatch"

TABLES="
episodes
podcasts
"

for T in $TABLES;
do
    psql $DB -c "DELETE FROM $T";
done
