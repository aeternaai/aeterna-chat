#!/usr/bin/env python3
"""
ChatGPT Export Validator
Validates that your ChatGPT export file is in the correct format before importing.

Usage:
    python validate_chatgpt_export.py conversations.json
"""

import json
import sys
from pathlib import Path


def validate_chatgpt_export(file_path: str) -> bool:
    """
    Validate ChatGPT export file format.

    Args:
        file_path: Path to conversations.json

    Returns:
        True if valid, False otherwise
    """
    print(f"🔍 Validating ChatGPT export file: {file_path}\n")

    # Check file exists
    if not Path(file_path).exists():
        print(f"✗ Error: File not found: {file_path}")
        return False

    print("✓ File exists")

    # Check file is readable
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
    except json.JSONDecodeError as e:
        print(f"✗ Error: Invalid JSON format: {e}")
        return False
    except Exception as e:
        print(f"✗ Error reading file: {e}")
        return False

    print("✓ Valid JSON format")

    # Check data structure
    conversations = data if isinstance(data, list) else [data]

    print(f"✓ Found {len(conversations)} conversation(s)")

    # Validate each conversation
    valid_count = 0
    empty_count = 0
    invalid_count = 0

    for idx, conv in enumerate(conversations, 1):
        if not isinstance(conv, dict):
            print(f"  [{idx}] ✗ Invalid: Not a dictionary")
            invalid_count += 1
            continue

        title = conv.get('title', f'Conversation {idx}')
        mapping = conv.get('mapping', {})

        if not mapping:
            print(f"  [{idx}] ⚠ Empty: '{title}' (no messages)")
            empty_count += 1
            continue

        # Count messages
        message_count = 0
        for node_id, node in mapping.items():
            message = node.get('message')
            if message and message.get('content') and message['content'].get('parts'):
                message_count += 1

        if message_count == 0:
            print(f"  [{idx}] ⚠ Empty: '{title}' (no valid messages)")
            empty_count += 1
        else:
            print(f"  [{idx}] ✓ Valid: '{title}' ({message_count} messages)")
            valid_count += 1

    # Summary
    print(f"\n{'='*60}")
    print("📊 Validation Summary:")
    print(f"  • Valid conversations: {valid_count}")
    print(f"  • Empty conversations: {empty_count}")
    print(f"  • Invalid conversations: {invalid_count}")
    print(f"  • Total: {len(conversations)}")
    print(f"{'='*60}\n")

    if valid_count == 0:
        print("✗ No valid conversations found to import")
        return False

    if invalid_count > 0:
        print("⚠ Warning: Some conversations are invalid and will be skipped")

    if empty_count > 0:
        print("⚠ Warning: Some conversations are empty and will be skipped")

    print(f"\n✓ Ready to import {valid_count} conversation(s)!")
    print("\nNext step:")
    print(f"  python3 chatgpt_to_jan_importer.py --chatgpt-export {file_path}")

    return True


def main():
    """Main entry point."""
    if len(sys.argv) != 2:
        print("Usage: python validate_chatgpt_export.py conversations.json")
        sys.exit(1)

    file_path = sys.argv[1]

    try:
        valid = validate_chatgpt_export(file_path)
        sys.exit(0 if valid else 1)
    except Exception as e:
        print(f"\n✗ Unexpected error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == '__main__':
    main()
