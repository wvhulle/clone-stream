#!/usr/bin/env bash
set -euo pipefail

echo "🚀 Running performance regression check..."
echo "ℹ️  Using separate target directories to preserve build caches"

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo "❌ Not in a git repository"
    exit 1
fi

# Check if there are any staged changes
if ! git diff --cached --quiet; then
    echo "📊 Benchmarking current staged changes..."
    
    # Create a temporary directory for benchmark results
    TEMP_DIR=$(mktemp -d)
    trap "rm -rf $TEMP_DIR" EXIT
    
    # Run benchmark on current staged changes and save baseline
    # Use normal target directory to preserve main build cache
    cargo bench --bench performance quick --quiet -- --save-baseline new > "$TEMP_DIR/new.out" 2>&1
    
    # Stash staged changes and checkout previous commit
    git stash push -q --keep-index -m "pre-commit-benchmark-temp"
    
    # Check if we have a previous commit to compare against
    if git rev-parse --verify HEAD~1 >/dev/null 2>&1; then
        echo "📊 Benchmarking previous commit (HEAD~1)..."
        
        git checkout HEAD~1 -q
        
        # Run benchmark on previous version with separate target directory
        # This preserves the main build cache and avoids invalidation
        CARGO_TARGET_DIR=target-benchmark-old cargo bench --bench performance quick --quiet -- --save-baseline old > "$TEMP_DIR/old.out" 2>&1
        
        # Return to original state - this is now done above
        # git checkout - -q
        # git stash pop -q
        
        echo "🔍 Comparing performance..."
        
        # Compare benchmarks - run new version with old baseline from separate target dir
        git checkout - -q
        git stash pop -q
        
        # Parse benchmark results and compare
        OLD_TIME=$(grep "time:" "$TEMP_DIR/old.out" | tail -1 | sed 's/.*time: *\[\([0-9.]*\) \([a-z]*\).*/\1 \2/' | head -1)
        NEW_TIME=$(grep "time:" "$TEMP_DIR/new.out" | tail -1 | sed 's/.*time: *\[\([0-9.]*\) \([a-z]*\).*/\1 \2/' | head -1)
        
        if [ -n "$OLD_TIME" ] && [ -n "$NEW_TIME" ]; then
            OLD_VAL=$(echo "$OLD_TIME" | cut -d' ' -f1)
            OLD_UNIT=$(echo "$OLD_TIME" | cut -d' ' -f2)
            NEW_VAL=$(echo "$NEW_TIME" | cut -d' ' -f1)
            NEW_UNIT=$(echo "$NEW_TIME" | cut -d' ' -f2)
            
            echo "Previous: $OLD_TIME, Current: $NEW_TIME"
            
            # Simple regression check: if new time is >20% higher, fail
            if [ "$OLD_UNIT" = "$NEW_UNIT" ]; then
                THRESHOLD=$(echo "$OLD_VAL * 1.2" | bc -l 2>/dev/null || echo "0")
                if [ "$THRESHOLD" != "0" ] && [ "$(echo "$NEW_VAL > $THRESHOLD" | bc -l 2>/dev/null || echo "0")" = "1" ]; then
                    PERCENT=$(echo "scale=1; ($NEW_VAL - $OLD_VAL) / $OLD_VAL * 100" | bc -l 2>/dev/null || echo "unknown")
                    echo "❌ Performance regression detected: +${PERCENT}% slower"
                    echo "💡 If this regression is expected, you can skip this check with:"
                    echo "   git commit --no-verify"
                    exit 1
                else
                    PERCENT=$(echo "scale=1; ($OLD_VAL - $NEW_VAL) / $OLD_VAL * 100" | bc -l 2>/dev/null || echo "0")
                    if [ "$(echo "$PERCENT > 0" | bc -l 2>/dev/null || echo "0")" = "1" ]; then
                        echo "✅ Performance improved: -${PERCENT}% faster"
                    else
                        echo "✅ No significant performance change"
                    fi
                fi
            else
                echo "⚠️  Cannot compare different units: $OLD_UNIT vs $NEW_UNIT"
            fi
        else
            echo "⚠️  Could not parse benchmark results for comparison"
        fi
    else
        echo "ℹ️  No previous commit to compare against (initial commit)"
        git stash pop -q
        exit 0
    fi
else
    echo "ℹ️  No staged changes to benchmark"
    exit 0
fi