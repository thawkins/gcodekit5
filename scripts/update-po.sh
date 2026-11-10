#!/bin/bash
set -e

# Create po directory if it doesn't exist
mkdir -p po

# Extract strings from source code
# We look for gettext calls: gettext("..."), t!("...")
# Note: We need to define keywords for xgettext to recognize our macros.
# Every tracked Rust source outside tests/ is scanned (the settings crate
# has translatable descriptions too); git ls-files avoids relying on the
# shell's globstar option, which `**` needs to recurse.
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
git ls-files 'crates/*.rs' 'src/*.rs' | grep -v '/tests/' | \
    xargs xgettext --language=Rust --keyword=gettext --keyword=t! --keyword=t --keyword=_ \
    --from-code=UTF-8 --add-comments --sort-by-file --no-location \
    --package-name="GCodeKit5" --package-version="$VERSION" \
    --output=po/gcodekit5.pot

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
