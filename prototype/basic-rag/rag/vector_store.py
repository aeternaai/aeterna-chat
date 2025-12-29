"""Vector store management using Qdrant and HuggingFace embeddings."""

from typing import List, Optional, Dict, Any
from pathlib import Path
from langchain_core.documents import Document
from langchain_huggingface import HuggingFaceEmbeddings
from langchain_qdrant import QdrantVectorStore
from qdrant_client import QdrantClient
from qdrant_client.models import Filter, FieldCondition, MatchValue
from config import settings


class QdrantStore:
    """
    Manages vector storage and retrieval using Qdrant with HuggingFace embeddings.
    
    Uses HuggingFace Transformers with nvidia/llama-nemotron-embed-1b-v2 model.
    """

    def __init__(
        self,
        embeddings_model_name: Optional[str] = None,
        client: Optional[QdrantClient] = None,
    ):
        """
        Initialize the QdrantStore.

        Args:
            embeddings_model_name: HuggingFace model name for embeddings
                                  (defaults to settings)
            client: Optional shared QdrantClient instance. If None, creates a new one.
        """
        self.embeddings_model_name = (
            embeddings_model_name or settings.EMBEDDINGS_MODEL_NAME
        )

        # Initialize or use shared Qdrant client
        if client is None:
            self.client = QdrantClient(path=str(settings.QDRANT_STORAGE_PATH))
            self._owns_client = True
        else:
            self.client = client
            self._owns_client = False

        # Initialize embeddings model
        print(f"🔧 Initializing HuggingFace embeddings model...")
        print(f"   Model: {self.embeddings_model_name}")
        print(f"   Note: Model will be downloaded automatically on first use")

        self.embeddings = HuggingFaceEmbeddings(
            model_name=self.embeddings_model_name,
            model_kwargs={
                'device': 'cpu',
                'trust_remote_code': True,
            },
            encode_kwargs={'normalize_embeddings': True},
        )

        print(f"✓ Embeddings model initialized")

    def add_documents(
        self,
        collection_name: str,
        documents: List[Document],
    ) -> List[str]:
        """
        Add documents to a Qdrant collection with embeddings.

        Args:
            collection_name: Name of the collection to add documents to
            documents: List of Document objects with content and metadata

        Returns:
            List of document IDs that were added

        Raises:
            ValueError: If collection doesn't exist or operation fails
        """
        if not documents:
            raise ValueError("No documents provided")

        print(f"📊 Adding {len(documents)} documents to collection '{collection_name}'...")

        try:
            # Initialize vector store for this collection
            vector_store = QdrantVectorStore(
                client=self.client,
                embedding=self.embeddings,
                collection_name=collection_name,
            )

            # Add documents (will automatically generate embeddings)
            print(f"   Generating embeddings...")
            doc_ids = vector_store.add_documents(documents)

            print(f"✓ Added {len(doc_ids)} documents to '{collection_name}'")
            return doc_ids

        except Exception as e:
            raise ValueError(
                f"Failed to add documents to collection '{collection_name}': {e}"
            )

    def similarity_search(
        self,
        collection_name: str,
        query: str,
        k: Optional[int] = None,
        score_threshold: Optional[float] = None,
        chat_id_filter: Optional[str] = None,
    ) -> List[Document]:
        """
        Perform similarity search in a collection.

        Args:
            collection_name: Name of the collection to search
            query: Query string
            k: Number of results to return (defaults to settings)
            score_threshold: Minimum similarity score (defaults to settings)
            chat_id_filter: Optional chat_id to filter results

        Returns:
            List of relevant Document objects with metadata and scores

        Raises:
            ValueError: If collection doesn't exist or search fails
        """
        k = k or settings.RETRIEVAL_TOP_K
        score_threshold = score_threshold or settings.RETRIEVAL_SCORE_THRESHOLD

        print(f"🔍 Searching collection '{collection_name}' for: '{query[:50]}...'")
        print(f"   Parameters: k={k}, threshold={score_threshold}")

        try:
            # Initialize vector store for this collection
            vector_store = QdrantVectorStore(
                client=self.client,
                embedding=self.embeddings,
                collection_name=collection_name,
            )

            # Build filter if chat_id is provided
            search_kwargs = {"k": k}
            if chat_id_filter:
                print(f"   Filtering by chat_id: {chat_id_filter}")
                # Create Qdrant filter for chat_id
                filter_condition = Filter(
                    must=[
                        FieldCondition(
                            key="metadata.chat_id",
                            match=MatchValue(value=chat_id_filter),
                        )
                    ]
                )
                search_kwargs["filter"] = filter_condition

            # Perform similarity search
            results = vector_store.similarity_search_with_score(
                query=query,
                **search_kwargs,
            )

            # Filter by score threshold
            filtered_results = [
                (doc, score)
                for doc, score in results
                if score >= score_threshold
            ]

            # Add scores to document metadata
            documents = []
            for doc, score in filtered_results:
                doc.metadata["similarity_score"] = float(score)
                documents.append(doc)

            print(f"✓ Found {len(documents)} relevant documents")
            return documents

        except Exception as e:
            raise ValueError(
                f"Failed to search collection '{collection_name}': {e}"
            )

    def get_collection_stats(self, collection_name: str) -> Dict[str, Any]:
        """
        Get statistics about a collection.

        Args:
            collection_name: Name of the collection

        Returns:
            Dictionary with collection statistics

        Raises:
            ValueError: If collection doesn't exist
        """
        try:
            collection_info = self.client.get_collection(
                collection_name=collection_name
            )

            return {
                "collection_name": collection_name,
                "points_count": collection_info.points_count,
                # In qdrant-client 1.16.2, vectors_count and indexed_vectors_count don't exist
                # Use points_count as each point has one vector
                "vectors_count": collection_info.points_count,
                "indexed_vectors_count": collection_info.points_count,
            }

        except Exception as e:
            raise ValueError(
                f"Failed to get stats for collection '{collection_name}': {e}"
            )

    def delete_documents(
        self, collection_name: str, document_ids: List[str]
    ) -> bool:
        """
        Delete specific documents from a collection.

        Args:
            collection_name: Name of the collection
            document_ids: List of document IDs to delete

        Returns:
            True if deletion was successful

        Raises:
            ValueError: If operation fails
        """
        try:
            # Delete points by IDs
            self.client.delete(
                collection_name=collection_name,
                points_selector=document_ids,
            )

            print(f"✓ Deleted {len(document_ids)} documents from '{collection_name}'")
            return True

        except Exception as e:
            raise ValueError(
                f"Failed to delete documents from '{collection_name}': {e}"
            )

    def cleanup(self):
        """Close Qdrant client connection."""
        try:
            if self._owns_client and hasattr(self, 'client') and self.client:
                self.client.close()
        except Exception as e:
            print(f"Warning: Error closing Qdrant connection: {e}")
