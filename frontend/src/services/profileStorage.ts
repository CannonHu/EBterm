import { readTextFile, writeTextFile, mkdir, exists } from '@tauri-apps/plugin-fs'
import { BaseDirectory } from '@tauri-apps/plugin-fs'
import type { ConnectionParams, SavedProfile, ProfileStorage } from '../types/ipc'

const PROFILE_DIR = 'embedded-debugger'
const PROFILE_FILE = 'profiles.json'
const MAX_PROFILES = 100

export class ProfileStorageService {
  private static instance: ProfileStorageService

  private constructor() {}

  static getInstance(): ProfileStorageService {
    if (!ProfileStorageService.instance) {
      ProfileStorageService.instance = new ProfileStorageService()
    }
    return ProfileStorageService.instance
  }

  private getProfileDirPath(): string {
    return PROFILE_DIR
  }

  private getProfileFilePath(): string {
    return `${PROFILE_DIR}/${PROFILE_FILE}`
  }

  private async ensureDirectory(): Promise<void> {
    const dirPath = this.getProfileDirPath()
    const dirExists = await exists(dirPath, { baseDir: BaseDirectory.AppData })
    
    if (!dirExists) {
      await mkdir(dirPath, { baseDir: BaseDirectory.AppData, recursive: true })
    }
  }

  async loadProfiles(): Promise<ProfileStorage> {
    try {
      await this.ensureDirectory()
      
      const filePath = this.getProfileFilePath()
      const fileExists = await exists(filePath, { baseDir: BaseDirectory.AppData })
      
      if (!fileExists) {
        return {}
      }

      const content = await readTextFile(filePath, { baseDir: BaseDirectory.AppData })
      return JSON.parse(content) as ProfileStorage
    } catch (error) {
      console.error('Failed to load profiles:', error)
      return {}
    }
  }

  async saveProfile(name: string, params: ConnectionParams): Promise<SavedProfile> {
    if (!name || name.trim().length === 0) {
      throw new Error('Profile name cannot be empty')
    }

    if (name.length > 100) {
      throw new Error('Profile name cannot exceed 100 characters')
    }

    const profiles = await this.loadProfiles()
    const profileCount = Object.keys(profiles).length

    if (!profiles[name] && profileCount >= MAX_PROFILES) {
      throw new Error(`Maximum ${MAX_PROFILES} profiles allowed. Please delete an existing profile first.`)
    }

    const savedProfile: SavedProfile = {
      name,
      params
    }

    profiles[name] = savedProfile

    await this.ensureDirectory()
    await writeTextFile(
      this.getProfileFilePath(),
      JSON.stringify(profiles, null, 2),
      { baseDir: BaseDirectory.AppData }
    )

    return savedProfile
  }

  async deleteProfile(name: string): Promise<void> {
    const profiles = await this.loadProfiles()
    
    if (!profiles[name]) {
      throw new Error(`Profile '${name}' not found`)
    }

    delete profiles[name]

    await this.ensureDirectory()
    await writeTextFile(
      this.getProfileFilePath(),
      JSON.stringify(profiles, null, 2),
      { baseDir: BaseDirectory.AppData }
    )
  }

  async listProfiles(): Promise<string[]> {
    const profiles = await this.loadProfiles()
    return Object.keys(profiles)
  }

  async getProfile(name: string): Promise<SavedProfile | undefined> {
    const profiles = await this.loadProfiles()
    return profiles[name]
  }
}

export const profileStorage = ProfileStorageService.getInstance()
