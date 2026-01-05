import { createFileRoute } from '@tanstack/react-router'
import SettingsMenu from '@/containers/SettingsMenu'
import HeaderPage from '@/containers/HeaderPage'
import { Card, CardItem } from '@/containers/Card'
import { useLangChainRag } from '@/hooks/useLangChainRag'
import type { SettingComponentProps } from '@janhq/core'
import { useTranslation } from '@/i18n/react-i18next-compat'
import { DynamicControllerSetting } from '@/containers/dynamicControllerSetting'
import { useEffect, useState, useCallback, useRef } from 'react'
import { useShallow } from 'zustand/react/shallow'

export const Route = createFileRoute('/settings/langchain-rag')({
  component: LangChainRagSettings,
})

function getConstraints(def: SettingComponentProps) {
  const props = def.controllerProps as Partial<{ min: number; max: number; step: number }>
  return {
    min: props.min ?? -Infinity,
    max: props.max ?? Infinity,
    step: props.step ?? 1,
  }
}

function clampValue(val: unknown, def: SettingComponentProps, currentValue: number): number {
  const num = typeof val === 'number' ? val : Number(val)
  if (!Number.isFinite(num)) return currentValue
  const { min, max, step } = getConstraints(def)
  const adjusted = step >= 1 ? Math.floor(num) : num
  return Math.max(min, Math.min(max, adjusted))
}

function LangChainRagSettings() {
  const { t } = useTranslation()
  const hookDefs = useLangChainRag((s) => s.settingsDefs)
  const loadDefs = useLangChainRag((s) => s.loadSettingsDefs)
  const [defs, setDefs] = useState<SettingComponentProps[]>([])

  useEffect(() => {
    loadDefs()
  }, [loadDefs])

  useEffect(() => {
    setDefs(hookDefs)
  }, [hookDefs])

  const sel = useLangChainRag(
    useShallow((s) => ({
      enabled: s.enabled,
      llmServerUrl: s.llmServerUrl,
      embeddingsModel: s.embeddingsModel,
      retrievalLimit: s.retrievalLimit,
      chunkSize: s.chunkSize,
      chunkOverlap: s.chunkOverlap,
      llmTemperature: s.llmTemperature,
      llmMaxTokens: s.llmMaxTokens,
      setEnabled: s.setEnabled,
      setLlmServerUrl: s.setLlmServerUrl,
      setEmbeddingsModel: s.setEmbeddingsModel,
      setRetrievalLimit: s.setRetrievalLimit,
      setChunkSize: s.setChunkSize,
      setChunkOverlap: s.setChunkOverlap,
      setLlmTemperature: s.setLlmTemperature,
      setLlmMaxTokens: s.setLlmMaxTokens,
    }))
  )

  const [localValues, setLocalValues] = useState<Record<string, string | number | boolean | string[]>>({})
  const timersRef = useRef<Record<string, ReturnType<typeof setTimeout>>>({})

  useEffect(() => {
    return () => {
      Object.values(timersRef.current).forEach((timer) => clearTimeout(timer))
    }
  }, [])

  const stateMap: Record<string, { current: unknown; setter: (v: any) => void }> = {
    enabled: { current: sel.enabled, setter: sel.setEnabled },
    llm_server_url: { current: sel.llmServerUrl, setter: sel.setLlmServerUrl },
    embeddings_model: { current: sel.embeddingsModel, setter: sel.setEmbeddingsModel },
    retrieval_limit: { current: sel.retrievalLimit, setter: sel.setRetrievalLimit },
    chunk_size: { current: sel.chunkSize, setter: sel.setChunkSize },
    chunk_overlap: { current: sel.chunkOverlap, setter: sel.setChunkOverlap },
    llm_temperature: { current: sel.llmTemperature, setter: sel.setLlmTemperature },
    llm_max_tokens: { current: sel.llmMaxTokens, setter: sel.setLlmMaxTokens },
  }

  const handleChange = useCallback(
    (key: string, val: string | number | boolean | string[]) => {
      const def = defs.find((d) => d.key === key)
      if (!def) return

      setLocalValues((prev) => ({ ...prev, [key]: val }))

      if (timersRef.current[key]) {
        clearTimeout(timersRef.current[key])
      }

      const isBool = typeof val === 'boolean'
      const delay = isBool ? 0 : 500

      timersRef.current[key] = setTimeout(() => {
        const mapped = stateMap[key]
        if (!mapped) return

        if (def.controllerType === 'input' && typeof val === 'number') {
          const clamped = clampValue(val, def, mapped.current as number)
          mapped.setter(clamped)
        } else {
          mapped.setter(val)
        }

        delete timersRef.current[key]
      }, delay)
    },
    [defs, stateMap]
  )

  const getDisplayValue = useCallback(
    (key: string): string | number | boolean | string[] => {
      if (key in localValues) {
        return localValues[key]
      }
      const mapped = stateMap[key]
      return mapped?.current as string | number | boolean | string[]
    },
    [localValues, stateMap]
  )

  return (
    <div className="flex flex-col h-full">
      <HeaderPage>
        <h1 className="font-medium">{t('common:settings')}</h1>
      </HeaderPage>
      <div className="flex h-full w-full flex-col sm:flex-row">
        <SettingsMenu />
        <div className="p-4 w-full h-[calc(100%-32px)] overflow-y-auto">
          <div className="flex flex-col justify-between gap-4 gap-y-3 w-full">
            <Card
              header={
                <div className="flex items-center justify-between mb-4">
                  <h1 className="text-main-view-fg font-medium text-base">
                    LangChain RAG Configuration
                  </h1>
                </div>
              }
            >
              {defs.length === 0 ? (
                <CardItem
                  title="Loading..."
                  description="Loading LangChain RAG settings..."
                />
              ) : (
                defs
                  .filter((def) => def && def.controllerProps)
                  .map((def) => {
                    const normalizedValue: string | number | boolean = (() => {
                      const val = getDisplayValue(def.key)
                      if (Array.isArray(val)) {
                        return val.join(',')
                      }
                      return val as string | number | boolean
                    })()

                    const props = {
                      value: normalizedValue,
                      placeholder: 'placeholder' in def.controllerProps ? def.controllerProps.placeholder : undefined,
                      type: 'type' in def.controllerProps ? (def.controllerProps.type as string) : undefined,
                      options: 'options' in def.controllerProps ? (def.controllerProps.options as { value: string | number; name: string }[]) : undefined,
                      input_actions: 'inputActions' in def.controllerProps ? (def.controllerProps.inputActions as string[]) : undefined,
                      rows: 'rows' in def.controllerProps ? (def.controllerProps.rows as number) : undefined,
                      min: 'min' in def.controllerProps ? (def.controllerProps.min as number) : undefined,
                      max: 'max' in def.controllerProps ? (def.controllerProps.max as number) : undefined,
                      step: 'step' in def.controllerProps ? (def.controllerProps.step as number) : undefined,
                      recommended: 'recommended' in def.controllerProps ? def.controllerProps.recommended : undefined,
                    }

                    const title = def.titleKey ? t(def.titleKey) : def.title
                    const description = def.descriptionKey ? t(def.descriptionKey) : def.description

                    return (
                      <CardItem
                        key={def.key}
                        title={title}
                        description={description}
                        actions={
                          <DynamicControllerSetting
                            controllerType={def.controllerType}
                            controllerProps={props}
                            onChange={(val) => handleChange(def.key, val)}
                          />
                        }
                      />
                    )
                  })
              )}
            </Card>
          </div>
        </div>
      </div>
    </div>
  )
}
