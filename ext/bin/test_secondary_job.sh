#!/bin/bash

# Set the temporary directory
TMP_DIR=$(mktemp -d)

# Function to check files in a subfolder
check_subfolder() {
  local subfolder="$1"
  local goal_s="$2"
  local max_stem="$3"

  # Iterate through files in the subfolder
  for file in "$TMP_DIR/$subfolder"/*; do
    if [[ -f "$file" ]]; then
      # Extract s and t values from the filename
      filename=$(basename "$file")
      IFS='_' read -ra parts <<< "$filename"
      file_s="${parts[0]}"
      file_t="${parts[1]}"

      # Check the condition t - s <= max_stem
      if (( file_t - file_s > max_stem )); then
        echo "Error: $subfolder contains files with stem > $max_stem"
        exit 1
      fi

      # If specific s values are required, check them
      if [[ "$subfolder" == "secondary_composites" && "$file_s" != "$goal_s" ]]; then
        echo "Error: $subfolder contains files with s != $goal_s"
        exit 1
      fi
      if [[ "$subfolder" == "secondary_intermediates" && "$file_s" != "$goal_s" ]]; then
        echo "Error: $subfolder contains files with s != $goal_s"
        exit 1
      fi
    fi
  done
}

# Run the cargo command
SECONDARY_JOB="3" cargo run --example secondary -- S_2 "$TMP_DIR" 10 6

# Check subfolders
check_subfolder "augmentation_qis" 0 10
check_subfolder "differentials" 0 10
check_subfolder "kernels" 0 10
check_subfolder "res_qis" 0 10
check_subfolder "secondary_composites" 3 10
check_subfolder "secondary_intermediates" 4 10

# Clean up temporary directory
rm -r "$TMP_DIR"

echo "CI script completed successfully"
