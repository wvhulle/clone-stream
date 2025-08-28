#!/usr/bin/env bash
set -euo pipefail

echo "🚀 Running performance regression check..."

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
    cargo bench --bench performance quick --quiet -- --save-baseline new > "$TEMP_DIR/new.out" 2>&1
    
    # Stash staged changes and checkout previous commit
    git stash push -q --keep-index -m "pre-commit-benchmark-temp"
    
    # Check if we have a previous commit to compare against
    if git rev-parse --verify HEAD~1 >/dev/null 2>&1; then
        echo "📊 Benchmarking previous commit (HEAD~1)..."
        
        git checkout HEAD~1 -q
        
        # Run benchmark on previous version and save baseline
        cargo bench --bench performance quick --quiet -- --save-baseline old > "$TEMP_DIR/old.out" 2>&1
        
        # Return to original state - this is now done above
        # git checkout - -q
        # git stash pop -q
        
        echo "🔍 Comparing performance..."
        
        # Compare benchmarks - run new version with old baseline to compare
        git checkout - -q
        git stash pop -q
        
        if cargo bench --bench performance quick --quiet -- --load-baseline old > "$TEMP_DIR/comparison.out" 2>&1; then
            echo "✅ No significant performance regression detected"
            
            # Show summary if there are improvements or minor changes
            if grep -q "Performance has" "$TEMP_DIR/comparison.out"; then
                grep "Performance has" "$TEMP_DIR/comparison.out" || true
            fi
            
            exit 0
        else
            echo "❌ Performance regression detected!"
            echo ""
            echo "Benchmark comparison output:"
            cat "$TEMP_DIR/comparison.out"
            echo ""
            echo "💡 If this regression is expected, you can skip this check with:"
            echo "   git commit --no-verify"
            exit 1
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