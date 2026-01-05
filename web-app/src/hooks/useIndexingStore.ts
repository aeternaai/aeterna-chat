import { create } from 'zustand'
import { listen } from '@tauri-apps/api/event'

export interface IndexingJobProgress {
  percent: number
  chunksProcessed: number
  totalChunks: number
  status: 'processing' | 'completed' | 'failed' | 'cancelled'
  error?: string
}

export interface IndexingState {
  indexingJobs: Record<string, IndexingJobProgress>
  updateIndexingProgress: (fileId: string, percent: number, processed: number, total: number) => void
  setIndexingStatus: (fileId: string, status: 'processing' | 'completed' | 'failed' | 'cancelled', error?: string) => void
  removeIndexingJob: (fileId: string) => void
  clearAllIndexingJobs: () => void
  getIndexingJob: (fileId: string) => IndexingJobProgress | undefined
  isIndexing: (fileId: string) => boolean
}

export const useIndexingStore = create<IndexingState>((set, get) => ({
  indexingJobs: {},

  updateIndexingProgress: (fileId, percent, processed, total) =>
    set((state) => ({
      indexingJobs: {
        ...state.indexingJobs,
        [fileId]: {
          ...state.indexingJobs[fileId],
          percent,
          chunksProcessed: processed,
          totalChunks: total,
        },
      },
    })),

  setIndexingStatus: (fileId, status, error) =>
    set((state) => ({
      indexingJobs: {
        ...state.indexingJobs,
        [fileId]: {
          ...state.indexingJobs[fileId],
          status,
          error,
        },
      },
    })),

  removeIndexingJob: (fileId) =>
    set((state) => {
      const newJobs = { ...state.indexingJobs }
      delete newJobs[fileId]
      return { indexingJobs: newJobs }
    }),

  clearAllIndexingJobs: () => set({ indexingJobs: {} }),

  getIndexingJob: (fileId) => get().indexingJobs[fileId],

  isIndexing: (fileId) => {
    const job = get().indexingJobs[fileId]
    return job?.status === 'processing'
  },
}))

// Setup event listeners for indexing progress
export function useIndexingEventSetup(): void {
  listen<any>('workspace-file-indexing-start', (event) => {
    const { file_id } = event.payload
    useIndexingStore.getState().setIndexingStatus(file_id, 'processing')
  }).catch((err) => console.error('[Indexing] Failed to listen for start event:', err))

  listen<any>('workspace-file-indexing-progress', (event) => {
    const { file_id, percent, processed, total } = event.payload
    useIndexingStore.getState().updateIndexingProgress(file_id, percent, processed, total)
  }).catch((err) => console.error('[Indexing] Failed to listen for progress event:', err))

  listen<any>('workspace-file-indexing-complete', (event) => {
    const { file_id } = event.payload
    useIndexingStore.getState().setIndexingStatus(file_id, 'completed')
  }).catch((err) => console.error('[Indexing] Failed to listen for complete event:', err))

  listen<any>('workspace-file-indexing-error', (event) => {
    const { file_id, error } = event.payload
    useIndexingStore.getState().setIndexingStatus(file_id, 'failed', error)
  }).catch((err) => console.error('[Indexing] Failed to listen for error event:', err))

  listen<any>('workspace-file-indexing-cancelled', (event) => {
    const { file_id } = event.payload
    useIndexingStore.getState().setIndexingStatus(file_id, 'cancelled')
  }).catch((err) => console.error('[Indexing] Failed to listen for cancelled event:', err))
}
