import fs from 'fs'
import path from 'path'
import { getLoggingService, ComponentLogger } from './LoggingService'

/**
 * Service for viewing and managing log files
 */
export class LogViewerService {
  private static instance: LogViewerService
  private logDir: string
  private logger: ComponentLogger

  private constructor() {
    this.logDir = getLoggingService().getLogDirectory()
    this.logger = getLoggingService().createLogger('LogViewerService')
  }

  /**
   * Get the singleton instance of the logging service
   */
  public static getInstance(): LogViewerService {
    if (!LogViewerService.instance) {
      LogViewerService.instance = new LogViewerService()
    }
    return LogViewerService.instance
  }

  /**
   * Get a list of available log files
   */
  public getLogFiles(): string[] {
    try {
      return fs
        .readdirSync(this.logDir)
        .filter((file) => file.endsWith('.log'))
        .sort((a, b) => {
          // Sort by modification time, newest first
          const statA = fs.statSync(path.join(this.logDir, a))
          const statB = fs.statSync(path.join(this.logDir, b))
          return statB.mtime.getTime() - statA.mtime.getTime()
        })
    } catch (error) {
      this.logger.error('Error getting log files', {
        error: error instanceof Error ? error.message : String(error)
      })
      return []
    }
  }

  /**
   * Read a log file
   * @param fileName Name of the log file
   * @param maxLines Maximum number of lines to read (most recent)
   */
  public readLogFile(fileName: string, maxLines = 1000): string[] {
    try {
      const filePath = path.join(this.logDir, fileName)

      // Check if file exists
      if (!fs.existsSync(filePath)) {
        throw new Error(`Log file ${fileName} not found`)
      }

      // Read file content
      const content = fs.readFileSync(filePath, 'utf-8')

      // Split into lines and take the most recent ones
      const lines = content.split('\n').filter((line) => line.trim() !== '')
      return lines.slice(-maxLines)
    } catch (error) {
      this.logger.error(`Error reading log file ${fileName}`, {
        error: error instanceof Error ? error.message : String(error)
      })
      return []
    }
  }

  /**
   * Get the path to the log directory
   */
  public getLogDirectory(): string {
    return this.logDir
  }

  /**
   * Clear all log files except the current ones
   */
  public clearOldLogs(): string[] {
    try {
      const currentFiles = ['zelan.log', 'error.log']
      const deletedFiles: string[] = []

      fs.readdirSync(this.logDir)
        .filter((file) => file.endsWith('.log') && !currentFiles.includes(file))
        .forEach((file) => {
          try {
            fs.unlinkSync(path.join(this.logDir, file))
            deletedFiles.push(file)
          } catch (error) {
            this.logger.error(`Error deleting log file ${file}`, {
              error: error instanceof Error ? error.message : String(error)
            })
          }
        })

      return deletedFiles
    } catch (error) {
      this.logger.error('Error clearing old logs', {
        error: error instanceof Error ? error.message : String(error)
      })
      return []
    }
  }
}

// Singleton accessor function
let logViewerServiceInstance: LogViewerService | null = null

export function getLogViewerService(): LogViewerService {
  if (!logViewerServiceInstance) {
    logViewerServiceInstance = LogViewerService.getInstance()
  }
  return logViewerServiceInstance
}
