#!/bin/bash
set -e

# Create po directory if it doesn't exist
mkdir -p po

# Extract strings from source code
# We look for gettext calls: gettext("..."), t!("...")
# Note: We need to define keywords for xgettext to recognize our macros
xgettext --language=Rust --keyword=gettext --keyword=t! --keyword=_ \
    --add-comments --sort-output --no-location \
    --package-name="GCodeKit5" --package-version="0.1.0" \
    --output=po/gcodekit5.pot \
    crates/gcodekit5-ui/src/*.rs crates/gcodekit5-ui/src/**/*.rs

echo "Updated po/gcodekit5.pot"

# Update or initialize PO files for each language
LANGS=("fr" "de" "es" "pt" "it")

for lang in "${LANGS[@]}"; do
    if [ -f "po/$lang.po" ]; then
        echo "Updating $lang.po..."
        msgmerge --update --backup=none "po/$lang.po" po/gcodekit5.pot
    else
        echo "Initializing $lang.po..."
        msginit --input=po/gcodekit5.pot --output="po/$lang.po" --locale="$lang" --no-translator
    fi
done

echo "Done."
