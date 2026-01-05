import { create } from 'zustand'
import { ExtensionManager } from '@/lib/extension'
import { ExtensionTypeEnum, type RAGExtension, type SettingComponentProps } from '@janhq/core'

export type LangChainRagSettings = {
  enabled: boolean
  llmServerUrl: string
  embeddingsModel: string
  retrievalLimit: number
  chunkSize: number
  chunkOverlap: number
  llmTemperature: number
  llmMaxTokens: number
}

type LangChainRagStore = LangChainRagSettings & {
  settingsDefs: SettingComponentProps[]
  loadSettingsDefs: () => Promise<void>
  setEnabled: (v: boolean) => void
  setLlmServerUrl: (v: string) => void
  setEmbeddingsModel: (v: string) => void
  setRetrievalLimit: (v: number) => void
  setChunkSize: (v: number) => void
  setChunkOverlap: (v: number) => void
  setLlmTemperature: (v: number) => void
  setLlmMaxTokens: (v: number) => void
}

const getLangChainExtension = (): RAGExtension | undefined => {
  try {
    return ExtensionManager.getInstance().get<RAGExtension>(ExtensionTypeEnum.LangChainRAG)
  } catch (error) {
    console.debug('Failed to get LangChain RAG extension:', error)
    return undefined
  }
}

export const useLangChainRag = create<LangChainRagStore>()((set) => ({
  enabled: false,
  llmServerUrl: 'http://127.0.0.1:8080/v1',
  embeddingsModel: 'sentence-transformers/all-MiniLM-L6-v2',
  retrievalLimit: 3,
  chunkSize: 512,
  chunkOverlap: 64,
  llmTemperature: 0.7,
  llmMaxTokens: 1024,
  settingsDefs: [],
  
  loadSettingsDefs: async () => {
    const ext = getLangChainExtension()
    if (!ext?.getSettings) return
    try {
      const defs = await ext.getSettings()
      if (Array.isArray(defs)) set({ settingsDefs: defs })
    } catch (e) {
      console.debug('Failed to load LangChain RAG settings defs:', e)
    }
  },
  
  setEnabled: async (v) => {
    const ext = getLangChainExtension()
    if (ext?.onSettingUpdate) {
      ext.onSettingUpdate('enabled', !!v)
    }
    set((s) => ({
      enabled: v,
      settingsDefs: s.settingsDefs.map((d) =>
        d.key === 'enabled'
          ? ({ ...d, controllerProps: { ...d.controllerProps, value: !!v } } as SettingComponentProps)
          : d
      ),
    }))
  },
  
  setLlmServerUrl: async (val) => {
    const ext = getLangChainExtension()
    if (ext?.onSettingUpdate) {
      ext.onSettingUpdate('llm_server_url', val)
    }
    set((s) => ({
      llmServerUrl: val,
      settingsDefs: s.settingsDefs.map((d) =>
        d.key === 'llm_server_url'
          ? ({ ...d, controllerProps: { ...d.controllerProps, value: val } } as SettingComponentProps)
          : d
      ),
    }))
  },
  
  setEmbeddingsModel: async (val) => {
    const ext = getLangChainExtension()
    if (ext?.onSettingUpdate) {
      ext.onSettingUpdate('embeddings_model', val)
    }
    set((s) => ({
      embeddingsModel: val,
      settingsDefs: s.settingsDefs.map((d) =>
        d.key === 'embeddings_model'
          ? ({ ...d, controllerProps: { ...d.controllerProps, value: val } } as SettingComponentProps)
          : d
      ),
    }))
  },
  
  setRetrievalLimit: async (val) => {
    const ext = getLangChainExtension()
    if (ext?.onSettingUpdate) {
      ext.onSettingUpdate('retrieval_limit', val)
    }
    set((s) => ({
      retrievalLimit: val,
      settingsDefs: s.settingsDefs.map((d) =>
        d.key === 'retrieval_limit'
          ? ({ ...d, controllerProps: { ...d.controllerProps, value: val } } as SettingComponentProps)
          : d
      ),
    }))
  },
  
  setChunkSize: async (val) => {
    const ext = getLangChainExtension()
    if (ext?.onSettingUpdate) {
      ext.onSettingUpdate('chunk_size', val)
    }
    set((s) => ({
      chunkSize: val,
      settingsDefs: s.settingsDefs.map((d) =>
        d.key === 'chunk_size'
          ? ({ ...d, controllerProps: { ...d.controllerProps, value: val } } as SettingComponentProps)
          : d
      ),
    }))
  },
  
  setChunkOverlap: async (val) => {
    const ext = getLangChainExtension()
    if (ext?.onSettingUpdate) {
      ext.onSettingUpdate('chunk_overlap', val)
    }
    set((s) => ({
      chunkOverlap: val,
      settingsDefs: s.settingsDefs.map((d) =>
        d.key === 'chunk_overlap'
          ? ({ ...d, controllerProps: { ...d.controllerProps, value: val } } as SettingComponentProps)
          : d
      ),
    }))
  },
  
  setLlmTemperature: async (val) => {
    const ext = getLangChainExtension()
    if (ext?.onSettingUpdate) {
      ext.onSettingUpdate('llm_temperature', val)
    }
    set((s) => ({
      llmTemperature: val,
      settingsDefs: s.settingsDefs.map((d) =>
        d.key === 'llm_temperature'
          ? ({ ...d, controllerProps: { ...d.controllerProps, value: val } } as SettingComponentProps)
          : d
      ),
    }))
  },
  
  setLlmMaxTokens: async (val) => {
    const ext = getLangChainExtension()
    if (ext?.onSettingUpdate) {
      ext.onSettingUpdate('llm_max_tokens', val)
    }
    set((s) => ({
      llmMaxTokens: val,
      settingsDefs: s.settingsDefs.map((d) =>
        d.key === 'llm_max_tokens'
          ? ({ ...d, controllerProps: { ...d.controllerProps, value: val } } as SettingComponentProps)
          : d
      ),
    }))
  },
}))
