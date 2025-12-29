"""Project and collection management for RAG system."""

from typing import Optional, List, Dict, Any
from qdrant_client import QdrantClient
from qdrant_client.models import Distance, VectorParams, CollectionInfo
from config import settings


class ProjectManager:
    """
    Manages Qdrant collections for projects and the global collection.
    
    Each project has its own collection for project-wide RAG context.
    Chats without projects use the 'global' collection.
    """

    def __init__(self, client: Optional[QdrantClient] = None):
        """Initialize the ProjectManager with Qdrant client.
        
        Args:
            client: Optional shared QdrantClient instance. If None, creates a new one.
        """
        if client is None:
            self.client = QdrantClient(path=str(settings.QDRANT_STORAGE_PATH))
            self._owns_client = True
        else:
            self.client = client
            self._owns_client = False
        self._vector_size = 2048  # nvidia/llama-nemotron-embed-1b-v2 embedding dimension

    def _collection_name_for_project(self, project_id: str) -> str:
        """Generate collection name for a project."""
        return f"project_{project_id}"

    def create_project_collection(
        self, project_id: str, project_name: str
    ) -> Dict[str, Any]:
        """
        Create a new collection for a project.

        Args:
            project_id: Unique identifier for the project
            project_name: Human-readable name for the project

        Returns:
            Dictionary with collection creation status and metadata

        Raises:
            ValueError: If project_id is empty or collection already exists
        """
        if not project_id or not project_id.strip():
            raise ValueError("project_id cannot be empty")

        collection_name = self._collection_name_for_project(project_id)

        # Check if collection already exists
        existing_collections = self.client.get_collections().collections
        if any(col.name == collection_name for col in existing_collections):
            raise ValueError(
                f"Collection for project '{project_id}' already exists"
            )

        # Create collection with vector configuration
        self.client.create_collection(
            collection_name=collection_name,
            vectors_config=VectorParams(
                size=self._vector_size, distance=Distance.COSINE
            ),
        )

        # Store project metadata as collection payload
        # Note: Qdrant doesn't have collection-level metadata, so we'll track this
        # separately or in the first document. For simplicity, we return it here.
        metadata = {
            "collection_name": collection_name,
            "project_id": project_id,
            "project_name": project_name,
            "type": "project",
        }

        print(f"✓ Created collection: {collection_name} for project '{project_name}'")
        return metadata

    def ensure_global_collection(self) -> Dict[str, Any]:
        """
        Ensure the global collection exists, create if it doesn't.

        Returns:
            Dictionary with collection metadata
        """
        collection_name = "global"

        # Check if collection exists
        existing_collections = self.client.get_collections().collections
        collection_exists = any(
            col.name == collection_name for col in existing_collections
        )

        if not collection_exists:
            # Create global collection
            self.client.create_collection(
                collection_name=collection_name,
                vectors_config=VectorParams(
                    size=self._vector_size, distance=Distance.COSINE
                ),
            )
            print(f"✓ Created global collection")

        return {
            "collection_name": collection_name,
            "project_id": None,
            "project_name": "Global",
            "type": "global",
        }

    def list_collections(self) -> List[Dict[str, Any]]:
        """
        List all collections with their metadata and statistics.

        Returns:
            List of dictionaries containing collection information
        """
        collections_response = self.client.get_collections()
        collections_list = []

        for collection in collections_response.collections:
            # Get collection info for statistics
            try:
                collection_info = self.client.get_collection(
                    collection_name=collection.name
                )
                points_count = collection_info.points_count
                # In qdrant-client 1.16.2, vectors_count doesn't exist
                # Use points_count as each point has one vector
                vectors_count = collection_info.points_count
            except Exception:
                points_count = 0
                vectors_count = 0

            # Parse collection name to determine type
            if collection.name == "global":
                collection_data = {
                    "collection_name": collection.name,
                    "project_id": None,
                    "project_name": "Global",
                    "type": "global",
                    "points_count": points_count,
                    "vectors_count": vectors_count,
                }
            elif collection.name.startswith("project_"):
                project_id = collection.name.replace("project_", "")
                collection_data = {
                    "collection_name": collection.name,
                    "project_id": project_id,
                    "project_name": f"Project {project_id}",
                    "type": "project",
                    "points_count": points_count,
                    "vectors_count": vectors_count,
                }
            else:
                # Unknown collection type
                collection_data = {
                    "collection_name": collection.name,
                    "project_id": None,
                    "project_name": collection.name,
                    "type": "unknown",
                    "points_count": points_count,
                    "vectors_count": vectors_count,
                }

            collections_list.append(collection_data)

        return collections_list

    def get_collection_info(self, collection_name: str) -> Dict[str, Any]:
        """
        Get detailed information about a specific collection.

        Args:
            collection_name: Name of the collection

        Returns:
            Dictionary with collection information

        Raises:
            ValueError: If collection doesn't exist
        """
        try:
            collection_info = self.client.get_collection(
                collection_name=collection_name
            )
        except Exception as e:
            raise ValueError(f"Collection '{collection_name}' not found: {e}")

        # Parse collection type
        if collection_name == "global":
            collection_type = "global"
            project_id = None
            project_name = "Global"
        elif collection_name.startswith("project_"):
            collection_type = "project"
            project_id = collection_name.replace("project_", "")
            project_name = f"Project {project_id}"
        else:
            collection_type = "unknown"
            project_id = None
            project_name = collection_name

        return {
            "collection_name": collection_name,
            "project_id": project_id,
            "project_name": project_name,
            "type": collection_type,
            "points_count": collection_info.points_count,
            # In qdrant-client 1.16.2, vectors_count doesn't exist
            # Use points_count as each point has one vector
            "vectors_count": collection_info.points_count,
            "vector_size": self._vector_size,
        }

    def delete_collection(self, collection_name: str) -> bool:
        """
        Delete a collection.

        Args:
            collection_name: Name of the collection to delete

        Returns:
            True if deletion was successful

        Raises:
            ValueError: If collection doesn't exist
        """
        try:
            self.client.delete_collection(collection_name=collection_name)
            print(f"✓ Deleted collection: {collection_name}")
            return True
        except Exception as e:
            raise ValueError(f"Failed to delete collection '{collection_name}': {e}")

    def collection_exists(self, collection_name: str) -> bool:
        """
        Check if a collection exists.

        Args:
            collection_name: Name of the collection to check

        Returns:
            True if collection exists, False otherwise
        """
        collections = self.client.get_collections().collections
        return any(col.name == collection_name for col in collections)

    def cleanup(self):
        """Close Qdrant client connection."""
        try:
            if self._owns_client and hasattr(self, 'client') and self.client:
                self.client.close()
        except Exception as e:
            print(f"Warning: Error closing Qdrant connection: {e}")
