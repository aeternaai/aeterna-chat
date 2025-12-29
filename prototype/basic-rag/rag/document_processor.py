"""Document processing and chunking for RAG system."""

import os
from pathlib import Path
from typing import List, Optional
from datetime import datetime
from langchain_core.documents import Document
from langchain_text_splitters import RecursiveCharacterTextSplitter
from langchain_community.document_loaders import (
    PyPDFLoader,
    TextLoader,
    Docx2txtLoader,
)
from config import settings


class DocumentProcessor:
    """
    Handles document loading, chunking, and metadata enrichment.
    
    Supports PDF, DOCX, and TXT files with configurable chunking strategy.
    """

    def __init__(
        self,
        chunk_size: Optional[int] = None,
        chunk_overlap: Optional[int] = None,
    ):
        """
        Initialize the DocumentProcessor.

        Args:
            chunk_size: Size of text chunks in characters (defaults to settings)
            chunk_overlap: Overlap between chunks in characters (defaults to settings)
        """
        self.chunk_size = chunk_size or settings.CHUNK_SIZE
        self.chunk_overlap = chunk_overlap or settings.CHUNK_OVERLAP

        # Initialize text splitter
        self.text_splitter = RecursiveCharacterTextSplitter(
            chunk_size=self.chunk_size,
            chunk_overlap=self.chunk_overlap,
            length_function=len,
            separators=["\n\n", "\n", " ", ""],
        )

    def _detect_file_type(self, file_path: Path) -> str:
        """
        Detect file type based on extension.

        Args:
            file_path: Path to the file

        Returns:
            File type as string ('pdf', 'docx', 'txt')

        Raises:
            ValueError: If file type is not supported
        """
        extension = file_path.suffix.lower()

        if extension == ".pdf":
            return "pdf"
        elif extension in [".docx", ".doc"]:
            return "docx"
        elif extension in [".txt", ".md", ".markdown"]:
            return "txt"
        else:
            raise ValueError(
                f"Unsupported file type: {extension}. "
                f"Supported types: .pdf, .docx, .txt, .md"
            )

    def _load_document(self, file_path: Path) -> List[Document]:
        """
        Load a document using the appropriate loader.

        Args:
            file_path: Path to the document file

        Returns:
            List of loaded Document objects

        Raises:
            ValueError: If file cannot be loaded
        """
        file_type = self._detect_file_type(file_path)

        try:
            if file_type == "pdf":
                loader = PyPDFLoader(str(file_path))
            elif file_type == "docx":
                loader = Docx2txtLoader(str(file_path))
            elif file_type == "txt":
                loader = TextLoader(str(file_path), encoding="utf-8")
            else:
                raise ValueError(f"Unsupported file type: {file_type}")

            documents = loader.load()
            return documents

        except Exception as e:
            raise ValueError(f"Failed to load document '{file_path}': {e}")

    def _enrich_metadata(
        self,
        chunks: List[Document],
        file_path: Path,
        project_id: Optional[str],
        project_name: Optional[str],
        chat_id: Optional[str],
    ) -> List[Document]:
        """
        Enrich document chunks with metadata.

        Args:
            chunks: List of document chunks
            file_path: Original file path
            project_id: Project identifier (None for global)
            project_name: Project name (None for global)
            chat_id: Chat identifier

        Returns:
            List of enriched Document objects
        """
        enriched_chunks = []
        timestamp = datetime.utcnow().isoformat()

        for idx, chunk in enumerate(chunks):
            # Create enriched metadata
            enriched_metadata = {
                "project_id": project_id,
                "project_name": project_name or "Global",
                "chat_id": chat_id,
                "source_file": file_path.name,
                "source_path": str(file_path),
                "chunk_index": idx,
                "timestamp": timestamp,
                "chunk_size": len(chunk.page_content),
            }

            # Merge with existing metadata
            if chunk.metadata:
                enriched_metadata.update(chunk.metadata)

            # Create new document with enriched metadata
            enriched_chunk = Document(
                page_content=chunk.page_content, metadata=enriched_metadata
            )
            enriched_chunks.append(enriched_chunk)

        return enriched_chunks

    def process_file(
        self,
        file_path: str | Path,
        project_id: Optional[str] = None,
        project_name: Optional[str] = None,
        chat_id: Optional[str] = None,
    ) -> List[Document]:
        """
        Process a file: load, chunk, and enrich with metadata.

        Args:
            file_path: Path to the file to process
            project_id: Project identifier (None for global collection)
            project_name: Project name (None for global collection)
            chat_id: Chat identifier (None for project-wide context)

        Returns:
            List of processed Document objects ready for embedding

        Raises:
            FileNotFoundError: If file doesn't exist
            ValueError: If file type is unsupported or processing fails
        """
        # Convert to Path object
        file_path = Path(file_path)

        # Validate file exists
        if not file_path.exists():
            raise FileNotFoundError(f"File not found: {file_path}")

        if not file_path.is_file():
            raise ValueError(f"Path is not a file: {file_path}")

        print(f"📄 Processing file: {file_path.name}")
        print(f"   File size: {file_path.stat().st_size / 1024:.2f} KB")

        # Load document
        print(f"   Loading document...")
        documents = self._load_document(file_path)
        print(f"   ✓ Loaded {len(documents)} page(s)")

        # Split into chunks
        print(f"   Chunking text (size={self.chunk_size}, overlap={self.chunk_overlap})...")
        chunks = self.text_splitter.split_documents(documents)
        print(f"   ✓ Created {len(chunks)} chunk(s)")

        # Enrich with metadata
        print(f"   Enriching metadata...")
        enriched_chunks = self._enrich_metadata(
            chunks=chunks,
            file_path=file_path,
            project_id=project_id,
            project_name=project_name,
            chat_id=chat_id,
        )

        print(f"✓ Processing complete: {len(enriched_chunks)} chunks ready")
        return enriched_chunks

    def get_supported_extensions(self) -> List[str]:
        """
        Get list of supported file extensions.

        Returns:
            List of supported extensions (with dot prefix)
        """
        return [".pdf", ".docx", ".doc", ".txt", ".md", ".markdown"]
