import { Model, SettingComponentProps } from '../types'
import { ModelManager } from './models'
import { fs } from './fs'
import { joinPath } from './core'

export enum ExtensionTypeEnum {
  Assistant = 'assistant',
  Conversational = 'conversational',
  Inference = 'inference',
  Model = 'model',
  SystemMonitoring = 'systemMonitoring',
  MCP = 'mcp',
  HuggingFace = 'huggingFace',
  Engine = 'engine',
  Hardware = 'hardware',
  RAG = 'rag',
  VectorDB = 'vectorDB',
  Router = 'router',
  Workspace = 'workspace',
}

export interface ExtensionType {
  type(): ExtensionTypeEnum | undefined
}

export interface Compatibility {
  platform: string[]
  version: string
}

/**
 * Represents a base extension.
 * This class should be extended by any class that represents an extension.
 */
export abstract class BaseExtension implements ExtensionType {
  protected settingFolderName = 'settings'
  protected settingFileName = 'settings.json'

  /** @type {string} Name of the extension. */
  name: string

  /** @type {string} Product Name of the extension. */
  productName?: string

  /** @type {string} The URL of the extension to load. */
  url: string

  /** @type {boolean} Whether the extension is activated or not. */
  active

  /** @type {string} Extension's description. */
  description

  /** @type {string} Extension's version. */
  version

  constructor(
    url: string,
    name: string,
    productName?: string,
    active?: boolean,
    description?: string,
    version?: string
  ) {
    this.name = name
    this.productName = productName
    this.url = url
    this.active = active
    this.description = description
    this.version = version
  }

  /**
   * Returns the type of the extension.
   * @returns {ExtensionType} The type of the extension
   * Undefined means its not extending any known extension by the application.
   */
  type(): ExtensionTypeEnum | undefined {
    return undefined
  }

  /**
   * Called when the extension is loaded.
   * Any initialization logic for the extension should be put here.
   */
  abstract onLoad(): void

  /**
   * Called when the extension is unloaded.
   * Any cleanup logic for the extension should be put here.
   */
  abstract onUnload(): void

  /**
   * The compatibility of the extension.
   * This is used to check if the extension is compatible with the current environment.
   * @property {Array} platform
   */
  compatibility(): Compatibility | undefined {
    return undefined
  }

  /**
   * Registers models - it persists in-memory shared ModelManager instance's data map.
   * @param models
   */
  async registerModels(models: Model[]): Promise<void> {
    for (const model of models) {
      ModelManager.instance().register(model)
    }
  }

  /**
   * Register settings for the extension.
   * @param settings
   * @returns
   */
  async registerSettings(settings: SettingComponentProps[]): Promise<void> {
    if (!this.name) {
      console.error('Extension name is not defined')
      return
    }

    settings.forEach((setting) => {
      setting.extensionName = this.name
    })
    try {
      const oldSettingsJson = localStorage.getItem(this.name)
      // Persists new settings
      if (oldSettingsJson) {
        const oldSettings = JSON.parse(oldSettingsJson)
        settings.forEach((setting) => {
          // Keep setting value
          if (setting.controllerProps && Array.isArray(oldSettings))
            setting.controllerProps.value =
              oldSettings.find((e: any) => e.key === setting.key)?.controllerProps?.value ??
              setting.controllerProps.value
          if ('options' in setting.controllerProps) {
            setting.controllerProps.options = setting.controllerProps.options?.length
              ? setting.controllerProps.options
              : oldSettings.find((e: any) => e.key === setting.key)?.controllerProps?.options
            if(!setting.controllerProps.options?.some(e => e.value === setting.controllerProps.value)) {
              setting.controllerProps.value = setting.controllerProps.options?.[0]?.value ?? setting.controllerProps.value
            }
          }
          if ('recommended' in setting.controllerProps) {
            const oldRecommended = oldSettings.find((e: any) => e.key === setting.key)
              ?.controllerProps?.recommended
            if (oldRecommended !== undefined && oldRecommended !== '') {
              setting.controllerProps.recommended = oldRecommended
            }
          }
        })
      }
      localStorage.setItem(this.name, JSON.stringify(settings))
    } catch (err) {
      console.error(err)
    }
  }

  /**
   * Get the setting value for the key.
   * @param key
   * @param defaultValue
   * @returns
   */
  async getSetting<T>(key: string, defaultValue: T) {
    const keySetting = (await this.getSettings()).find((setting) => setting.key === key)

    const value = keySetting?.controllerProps.value
    return (value as T) ?? defaultValue
  }

  onSettingUpdate<T>(key: string, value: T) {
    return
  }

  /**
   * Install the prerequisites for the extension.
   *
   * @returns {Promise<void>}
   */
  async install(): Promise<void> {
    return
  }

  /**
   * Get the settings for the extension.
   * @returns
   */
  async getSettings(): Promise<SettingComponentProps[]> {
    if (!this.name) return []

    try {
      const settingsString = localStorage.getItem(this.name)
      if (!settingsString) return []
      const settings: SettingComponentProps[] = JSON.parse(settingsString)
      return settings
    } catch (err) {
      console.warn(err)
      return []
    }
  }

  /**
   * Update the settings for the extension.
   * @param componentProps
   * @returns
   */
  async updateSettings(componentProps: Partial<SettingComponentProps>[]): Promise<void> {
    if (!this.name) return

    console.log(`[Extension:${this.name}] updateSettings called with:`, componentProps)

    const settings = await this.getSettings()

    let updatedSettings = settings.map((setting) => {
      const updatedSetting = componentProps.find(
        (componentProp) => componentProp.key === setting.key
      )
      if (updatedSetting && updatedSetting.controllerProps) {
        setting.controllerProps.value = updatedSetting.controllerProps.value
      }
      return setting
    })

    if (!updatedSettings.length) updatedSettings = componentProps as SettingComponentProps[]

    // Save to localStorage (always, for web compatibility)
    localStorage.setItem(this.name, JSON.stringify(updatedSettings))
    console.log(`[Extension:${this.name}] Settings saved to localStorage`)

    // ALSO save to file system if available (Tauri/desktop mode)
    try {
      // Check if fs API is available
      if (globalThis.core?.api?.writeFileSync) {
        console.log(`[Extension:${this.name}] File system API available, persisting to settings.json...`)
        
        // Build path to extension's settings.json file
        // Extension URL is like: file://extensions/router-extension/index.js
        // We want: file://extensions/router-extension/settings.json
        const settingsPath = await joinPath([this.url.replace(/\/[^\/]+$/, ''), 'settings.json'])
        
        console.log(`[Extension:${this.name}] Writing settings to:`, settingsPath)
        
        // Convert settings to JSON format matching settings.json structure
        const settingsJson: Record<string, any> = {}
        updatedSettings.forEach(setting => {
          settingsJson[setting.key] = setting.controllerProps.value
        })
        
        await fs.writeFileSync(settingsPath, JSON.stringify(settingsJson, null, 2))
        console.log(`[Extension:${this.name}] ✅ Settings persisted to settings.json successfully`)
      } else {
        console.log(`[Extension:${this.name}] File system API not available (web mode), using localStorage only`)
      }
    } catch (error) {
      console.error(`[Extension:${this.name}] ⚠️  Failed to persist settings to file system:`, error)
      // Don't throw - localStorage save already succeeded
    }

    updatedSettings.forEach((setting) => {
      this.onSettingUpdate<typeof setting.controllerProps.value>(
        setting.key,
        setting.controllerProps.value
      )
    })
  }
}

/**
 * Workspace extension for managing workspaces and file references.
 * Provides abstraction layer for different storage implementations.
 * @extends BaseExtension
 */
export abstract class WorkspaceExtension extends BaseExtension {
  /**
   * Returns the type of the extension.
   * @returns {ExtensionTypeEnum} The type of the extension
   */
  override type(): ExtensionTypeEnum {
    return ExtensionTypeEnum.Workspace
  }

  /**
   * Creates a new workspace.
   * @abstract
   */
  abstract createWorkspace(workspace: any): Promise<void>

  /**
   * Updates an existing workspace.
   * @abstract
   */
  abstract updateWorkspace(workspace: any): Promise<void>

  /**
   * Deletes an existing workspace.
   * @abstract
   */
  abstract deleteWorkspace(workspaceId: string): Promise<void>

  /**
   * Retrieves a specific workspace by ID.
   * @abstract
   */
  abstract getWorkspace(workspaceId: string): Promise<any>

  /**
   * Retrieves all existing workspaces.
   * @abstract
   */
  abstract getWorkspaces(): Promise<any[]>

  /**
   * Adds a file reference to a workspace.
   * @abstract
   */
  abstract addFileToWorkspace(file: any): Promise<void>

  /**
   * Removes a file reference from a workspace.
   * @abstract
   */
  abstract removeFileFromWorkspace(fileId: string): Promise<void>

  /**
   * Updates a file reference in a workspace.
   * @abstract
   */
  abstract updateWorkspaceFile(file: any): Promise<void>

  /**
   * Retrieves a specific file reference by ID.
   * @abstract
   */
  abstract getWorkspaceFile(fileId: string): Promise<any>

  /**
   * Retrieves all file references for a specific workspace.
   * @abstract
   */
  abstract getWorkspaceFiles(workspaceId: string): Promise<any[]>

  /**
   * Validates that a file reference still points to an existing file.
   * @abstract
   */
  abstract validateFileReference(fileId: string): Promise<any>

  /**
   * Validates all file references in a workspace.
   * @abstract
   */
  abstract validateWorkspaceFiles(workspaceId: string): Promise<any[]>
}

