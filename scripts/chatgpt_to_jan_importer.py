#!/usr/bin/env python3
"""
ChatGPT to Jan Importer
Converts ChatGPT export data to Jan's format and imports conversations.

Usage:
    python chatgpt_to_jan_importer.py --chatgpt-export conversations.json

Requirements:
    pip install requests
"""

import json
import os
import sys
import argparse
import uuid
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any, Optional
import time


class ChatGPTToJanConverter:
    """Converts ChatGPT conversations to Jan format."""

    def __init__(self, jan_data_dir: Optional[str] = None):
        """
        Initialize converter.

        Args:
            jan_data_dir: Path to Jan data directory.
                         Defaults to ~/Library/Application Support/Jan/data on macOS
        """
        if jan_data_dir:
            self.jan_data_dir = Path(jan_data_dir)
        else:
            # Default macOS location
            self.jan_data_dir = Path.home() / "Library" / "Application Support" / "Jan" / "data"

        self.threads_dir = self.jan_data_dir / "threads"
        self.threads_dir.mkdir(parents=True, exist_ok=True)

        print(f"✓ Jan data directory: {self.jan_data_dir}")
        print(f"✓ Threads directory: {self.threads_dir}")

    def convert_chatgpt_export(self, chatgpt_export_path: str) -> None:
        """
        Convert ChatGPT export file to Jan format.

        Args:
            chatgpt_export_path: Path to ChatGPT conversations.json export file
        """
        print(f"\n📂 Loading ChatGPT export from: {chatgpt_export_path}")

        with open(chatgpt_export_path, 'r', encoding='utf-8') as f:
            chatgpt_data = json.load(f)

        # ChatGPT export can be either a list of conversations or a dict
        conversations = chatgpt_data if isinstance(chatgpt_data, list) else [chatgpt_data]

        print(f"✓ Found {len(conversations)} conversation(s) to import\n")

        imported_count = 0
        skipped_count = 0

        for idx, conversation in enumerate(conversations, 1):
            try:
                print(f"[{idx}/{len(conversations)}] Processing conversation...")
                self._import_conversation(conversation)
                imported_count += 1
                print(f"  ✓ Successfully imported\n")
            except Exception as e:
                print(f"  ✗ Error importing conversation: {e}\n")
                skipped_count += 1

        print(f"\n{'='*60}")
        print(f"✓ Import complete!")
        print(f"  • Imported: {imported_count}")
        print(f"  • Skipped: {skipped_count}")
        print(f"  • Total: {len(conversations)}")
        print(f"{'='*60}\n")

    def _import_conversation(self, conversation: Dict[str, Any]) -> None:
        """
        Import a single ChatGPT conversation.

        Args:
            conversation: ChatGPT conversation object
        """
        # Extract conversation metadata
        title = conversation.get('title', 'Untitled Conversation')
        create_time = conversation.get('create_time', time.time())
        update_time = conversation.get('update_time', create_time)

        # Generate unique thread ID
        thread_id = str(uuid.uuid4())

        # Convert timestamps to milliseconds
        created_ms = int(create_time * 1000) if create_time else int(time.time() * 1000)
        updated_ms = int(update_time * 1000) if update_time else created_ms

        # Extract model info from conversation
        model_slug = self._extract_model_from_conversation(conversation)

        # Create Jan thread object
        thread = {
            "id": thread_id,
            "object": "thread",
            "title": title[:200],  # Limit title length
            "assistants": [{
                "id": "chatgpt-import",
                "name": "ChatGPT (Imported)",
                "model": {
                    "id": model_slug,
                    "engine": "openai"
                },
                "instructions": "You are a helpful assistant."
            }],
            "created": created_ms,
            "updated": updated_ms,
            "metadata": {
                "imported_from": "chatgpt",
                "original_conversation_id": conversation.get('id', ''),
                "import_date": int(time.time() * 1000)
            }
        }

        # Create thread directory
        thread_dir = self.threads_dir / thread_id
        thread_dir.mkdir(parents=True, exist_ok=True)

        # Write thread.json
        thread_json_path = thread_dir / "thread.json"
        with open(thread_json_path, 'w', encoding='utf-8') as f:
            json.dump(thread, f, indent=2, ensure_ascii=False)

        print(f"  • Title: {title}")
        print(f"  • Thread ID: {thread_id}")

        # Convert and write messages
        messages = self._extract_messages_from_conversation(conversation, thread_id)

        if messages:
            messages_jsonl_path = thread_dir / "messages.jsonl"
            with open(messages_jsonl_path, 'w', encoding='utf-8') as f:
                for message in messages:
                    f.write(json.dumps(message, ensure_ascii=False) + '\n')

            print(f"  • Messages: {len(messages)}")
        else:
            print(f"  • Messages: 0 (empty conversation)")

    def _extract_model_from_conversation(self, conversation: Dict[str, Any]) -> str:
        """
        Extract model information from ChatGPT conversation.

        Args:
            conversation: ChatGPT conversation object

        Returns:
            Model identifier string
        """
        # Try to find model from mapping or conversation metadata
        mapping = conversation.get('mapping', {})

        for node_id, node in mapping.items():
            message = node.get('message')
            if message and message.get('metadata'):
                model_slug = message['metadata'].get('model_slug')
                if model_slug:
                    return model_slug

        # Default fallback
        return "gpt-3.5-turbo"

    def _extract_messages_from_conversation(
        self,
        conversation: Dict[str, Any],
        thread_id: str
    ) -> List[Dict[str, Any]]:
        """
        Extract and convert messages from ChatGPT conversation.

        Args:
            conversation: ChatGPT conversation object
            thread_id: Jan thread ID

        Returns:
            List of Jan-formatted messages
        """
        messages = []
        mapping = conversation.get('mapping', {})

        if not mapping:
            return messages

        # Build conversation tree
        nodes = []
        for node_id, node in mapping.items():
            message = node.get('message')
            if message and message.get('content') and message['content'].get('parts'):
                nodes.append({
                    'id': node_id,
                    'parent': node.get('parent'),
                    'message': message
                })

        # Sort by creation time to maintain conversation order
        nodes.sort(key=lambda n: n['message'].get('create_time', 0))

        # Convert each message
        for node in nodes:
            message = node['message']

            # Extract role
            role = message.get('author', {}).get('role', 'user')
            if role == 'system':
                role = 'system'
            elif role == 'assistant':
                role = 'assistant'
            else:
                role = 'user'

            # Extract content
            content_parts = message.get('content', {}).get('parts', [])
            if not content_parts:
                continue

            # Combine all parts into text
            text_content = '\n'.join(str(part) for part in content_parts if part)

            if not text_content.strip():
                continue

            # Create Jan message
            jan_message = {
                "id": str(uuid.uuid4()),
                "object": "thread.message",
                "thread_id": thread_id,
                "role": role,
                "content": [{
                    "type": "text",
                    "text": {
                        "value": text_content,
                        "annotations": []
                    }
                }],
                "status": "ready",
                "created_at": int(message.get('create_time', time.time()) * 1000),
                "completed_at": int(message.get('create_time', time.time()) * 1000),
                "metadata": {
                    "original_message_id": message.get('id', ''),
                    "author_name": message.get('author', {}).get('name', '')
                }
            }

            messages.append(jan_message)

        return messages


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description='Import ChatGPT conversations into Jan',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Import from ChatGPT export file
  python chatgpt_to_jan_importer.py --chatgpt-export conversations.json

  # Specify custom Jan data directory
  python chatgpt_to_jan_importer.py --chatgpt-export conversations.json --jan-dir ~/custom/jan/path

How to export from ChatGPT:
  1. Go to ChatGPT (https://chat.openai.com)
  2. Click your profile icon → Settings
  3. Go to "Data controls" → "Export data"
  4. Wait for email with download link
  5. Download and extract conversations.json
        """
    )

    parser.add_argument(
        '--chatgpt-export',
        required=True,
        help='Path to ChatGPT conversations.json export file'
    )

    parser.add_argument(
        '--jan-dir',
        help='Path to Jan data directory (default: ~/Library/Application Support/Jan/data on macOS)'
    )

    args = parser.parse_args()

    # Validate export file exists
    if not os.path.exists(args.chatgpt_export):
        print(f"✗ Error: ChatGPT export file not found: {args.chatgpt_export}")
        sys.exit(1)

    # Initialize converter
    converter = ChatGPTToJanConverter(jan_data_dir=args.jan_dir)

    # Perform conversion
    try:
        converter.convert_chatgpt_export(args.chatgpt_export)
    except Exception as e:
        print(f"\n✗ Fatal error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == '__main__':
    main()
