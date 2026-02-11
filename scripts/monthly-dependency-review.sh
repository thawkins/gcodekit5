#!/bin/bash
# Monthly Dependency Review Script
# Run this script monthly to review and update dependencies
#
# Prerequisites:
#   cargo install cargo-outdated cargo-audit
#
# Usage:
#   ./scripts/monthly-dependency-review.sh [--update]
#
# Options:
#   --update    Apply safe updates (minor/patch versions)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
REPORT_DIR="$PROJECT_ROOT/target/temp"
REPORT_FILE="$REPORT_DIR/dependency-review-$(date +%Y-%m).md"

cd "$PROJECT_ROOT"

# Ensure report directory exists
mkdir -p "$REPORT_DIR"

echo "=========================================="
echo "  Monthly Dependency Review"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="
echo ""

# Check for required tools
check_tool() {
    if ! command -v "$1" &> /dev/null; then
        echo "⚠️  $1 not found. Install with: cargo install $1"
        return 1
    fi
    return 0
}

MISSING_TOOLS=0
check_tool cargo-outdated || MISSING_TOOLS=1
check_tool cargo-audit || MISSING_TOOLS=1

if [ $MISSING_TOOLS -eq 1 ]; then
    echo ""
    echo "Please install missing tools and re-run."
    exit 1
fi

# Start report
cat > "$REPORT_FILE" << EOF
# Monthly Dependency Review - $(date '+%B %Y')

**Date**: $(date '+%Y-%m-%d')
**Reviewer**: $(git config user.name || echo "Unknown")

## Summary

EOF

echo "📦 Step 1: Checking for outdated dependencies..."
echo ""
echo "### Outdated Dependencies" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo '```' >> "$REPORT_FILE"

if cargo outdated --root-deps-only 2>&1 | tee -a "$REPORT_FILE"; then
    echo "✅ Outdated check complete"
else
    echo "⚠️  cargo-outdated encountered issues"
fi

echo '```' >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

echo ""
echo "🔒 Step 2: Running security audit..."
echo ""
echo "### Security Audit" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo '```' >> "$REPORT_FILE"

if cargo audit 2>&1 | tee -a "$REPORT_FILE"; then
    echo "✅ No security vulnerabilities found"
    echo "" >> "$REPORT_FILE"
    echo "✅ No security vulnerabilities found" >> "$REPORT_FILE"
else
    echo "⚠️  Security issues detected - review required!"
fi

echo '```' >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

echo ""
echo "📊 Step 3: Checking dependency tree for duplicates..."
echo ""
echo "### Duplicate Dependencies" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo '```' >> "$REPORT_FILE"

# Find duplicate crates (different versions of same crate)
cargo tree --duplicates 2>&1 | head -50 | tee -a "$REPORT_FILE"

echo '```' >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

echo ""
echo "📋 Step 4: Generating update recommendations..."
echo ""

cat >> "$REPORT_FILE" << 'EOF'
## Recommended Actions

### Priority 1: Security Updates
Apply immediately if any vulnerabilities are found above.

### Priority 2: Major Version Updates
Review changelogs before updating:
- Check for breaking API changes
- Run full test suite after update
- Update any affected code

### Priority 3: Minor/Patch Updates
Generally safe to apply:
```bash
cargo update
```

## Update Procedure

1. Create a new branch:
   ```bash
   git checkout -b chore/monthly-deps-YYYY-MM
   ```

2. Apply updates:
   ```bash
   # For safe updates (minor/patch)
   cargo update
   
   # For specific major updates
   cargo update -p <package>@<version>
   ```

3. Run tests:
   ```bash
   cargo test
   cargo clippy
   ```

4. Update CHANGELOG.md with dependency changes

5. Create PR for review

## Checklist

- [ ] Security audit passed (no vulnerabilities)
- [ ] Outdated dependencies reviewed
- [ ] Major updates evaluated for breaking changes
- [ ] cargo update applied for minor/patch
- [ ] All tests pass
- [ ] CHANGELOG.md updated
- [ ] PR created and reviewed

EOF

# Apply updates if requested
if [ "$1" == "--update" ]; then
    echo ""
    echo "🔄 Applying safe updates (minor/patch)..."
    cargo update
    echo "✅ Updates applied. Run 'cargo test' to verify."
fi

echo ""
echo "=========================================="
echo "  Review Complete!"
echo "=========================================="
echo ""
echo "📄 Report saved to: $REPORT_FILE"
echo ""
echo "Next steps:"
echo "  1. Review the report at $REPORT_FILE"
echo "  2. Address any security vulnerabilities immediately"
echo "  3. Evaluate major version updates"
echo "  4. Run: $0 --update (to apply safe updates)"
echo "  5. Run: cargo test (to verify changes)"
echo "  6. Update CHANGELOG.md"
echo "  7. Create PR for review"
echo ""
