import winston, { format, transports } from 'winston'
import { app } from 'electron'
import path from 'path'
import fs from 'fs'

// Define log levels with their colors
const logLevels = {
  error: 0,
  warn: 1,
  info: 2,
  http: 3,
  debug: 4,
  trace: 5
}

// Define custom format for console output (colorized)
const consoleFormat = format.combine(
  format.timestamp({ format: 'YYYY-MM-DD HH:mm:ss.SSS' }),
  format.colorize({ all: true }),
  format.printf(({ timestamp, level, message, component, ...meta }) => {
    const componentStr = component ? `[${component}] ` : ''
    const metaStr = Object.keys(meta).length ? ` ${JSON.stringify(meta)}` : ''
    return `${timestamp} ${level}: ${componentStr}${message}${metaStr}`
  })
)

// Define custom format for file output (JSON)
const fileFormat = format.combine(format.timestamp({ format: 'YYYY-MM-DD HH:mm:ss.SSS' }), format.json())

/**
 * Logging service for the application
 */
export class LoggingService {
  private static instance: LoggingService
  private logger: winston.Logger
  private logDir: string

  private constructor() {
    // Create logs directory in user data folder
    this.logDir = path.join(app.getPath('userData'), 'logs')
    fs.mkdirSync(this.logDir, { recursive: true })

    // Create the logger
    this.logger = winston.createLogger({
      levels: logLevels,
      level: process.env.NODE_ENV === 'production' ? 'info' : 'debug',
      defaultMeta: { service: 'zelan' },
      transports: [
        // Console transport
        new transports.Console({
          format: consoleFormat
        }),
        // File transport for all logs
        new transports.File({
          filename: path.join(this.logDir, 'zelan.log'),
          format: fileFormat,
          maxsize: 5242880, // 5MB
          maxFiles: 5,
          tailable: true
        }),
        // File transport for error logs
        new transports.File({
          filename: path.join(this.logDir, 'error.log'),
          level: 'error',
          format: fileFormat,
          maxsize: 5242880, // 5MB
          maxFiles: 5,
          tailable: true
        })
      ]
    })

    // Log startup
    this.logger.info('Logging service initialized', {
      component: 'LoggingService',
      logDir: this.logDir
    })
  }

  /**
   * Get the singleton instance of the logging service
   */
  public static getInstance(): LoggingService {
    if (!LoggingService.instance) {
      LoggingService.instance = new LoggingService()
    }
    return LoggingService.instance
  }

  /**
   * Log an error message
   */
  public error(message: string, meta?: Record<string, any>): void {
    this.logger.error(message, meta)
  }

  /**
   * Log a warning message
   */
  public warn(message: string, meta?: Record<string, any>): void {
    this.logger.warn(message, meta)
  }

  /**
   * Log an info message
   */
  public info(message: string, meta?: Record<string, any>): void {
    this.logger.info(message, meta)
  }

  /**
   * Log an HTTP request
   */
  public http(message: string, meta?: Record<string, any>): void {
    this.logger.http(message, meta)
  }

  /**
   * Log a debug message
   */
  public debug(message: string, meta?: Record<string, any>): void {
    this.logger.debug(message, meta)
  }

  /**
   * Log a trace message
   */
  public trace(message: string, meta?: Record<string, any>): void {
    this.logger.log('trace', message, meta)
  }

  /**
   * Get the full path to the log directory
   */
  public getLogDirectory(): string {
    return this.logDir
  }

  /**
   * Create a child logger for a specific component
   */
  public createLogger(component: string): ComponentLogger {
    return new ComponentLogger(this.logger, component)
  }
}

/**
 * Logger for a specific component
 */
export class ComponentLogger {
  private logger: winston.Logger
  private component: string

  constructor(logger: winston.Logger, component: string) {
    this.logger = logger
    this.component = component
  }

  /**
   * Log an error message
   */
  public error(message: string, meta?: Record<string, any>): void {
    this.logger.error(message, { component: this.component, ...meta })
  }

  /**
   * Log a warning message
   */
  public warn(message: string, meta?: Record<string, any>): void {
    this.logger.warn(message, { component: this.component, ...meta })
  }

  /**
   * Log an info message
   */
  public info(message: string, meta?: Record<string, any>): void {
    this.logger.info(message, { component: this.component, ...meta })
  }

  /**
   * Log an HTTP request
   */
  public http(message: string, meta?: Record<string, any>): void {
    this.logger.http(message, { component: this.component, ...meta })
  }

  /**
   * Log a debug message
   */
  public debug(message: string, meta?: Record<string, any>): void {
    this.logger.debug(message, { component: this.component, ...meta })
  }

  /**
   * Log a trace message
   */
  public trace(message: string, meta?: Record<string, any>): void {
    this.logger.log('trace', message, { component: this.component, ...meta })
  }
}

// Singleton accessor function
let loggingServiceInstance: LoggingService | null = null

export function getLoggingService(): LoggingService {
  if (!loggingServiceInstance) {
    loggingServiceInstance = LoggingService.getInstance()
  }
  return loggingServiceInstance
}

// For testing and reset
export function _resetLoggingService(): void {
  loggingServiceInstance = null
}
