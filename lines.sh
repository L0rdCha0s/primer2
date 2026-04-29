RS=$(find . -type f \( -name '*.rs'  \) -not -path './ci/*' -not -path './backend/target/*' -not -path './backend/vendor/*' -exec cat {} + | wc -l | awk '{print $1}'); \
TSX=$(find . -type f \( -name '*.tsx' \) -not -path './ci/*' -not -path './backend/target/*' -not -path './backend/vendor/*' -exec cat {} + | wc -l | awk '{print $1}'); \
TS=$(find . -type f \( -name '*.ts' \) -not -path '*/node_modules/*' -not -path './ci/*' -not -path './backend/target/*' -not -path './backend/vendor/*' -exec cat {} + | wc -l | awk '{print $1}'); \
printf 'RS: %s,  TS: %s, TSX: %s  Total: %s\n' "$RS" "$TS" "$TSX" "$((RS+TSX+TS))"
