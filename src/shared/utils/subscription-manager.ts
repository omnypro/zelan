import { Subscription } from 'rxjs'

/**
 * Utility class for managing RxJS subscriptions.
 * 
 * This helps ensure proper cleanup of subscriptions to prevent memory leaks.
 * Use this in any class that maintains RxJS subscriptions.
 */
export class SubscriptionManager {
  private subscriptions: Subscription[] = []
  
  /**
   * Add a subscription to be managed
   * @param subscription The subscription to add
   * @returns The subscription that was added (for chaining)
   */
  add(subscription: Subscription): Subscription {
    this.subscriptions.push(subscription)
    return subscription
  }
  
  /**
   * Remove and unsubscribe a specific subscription
   * @param subscription The subscription to remove
   */
  remove(subscription: Subscription): void {
    const index = this.subscriptions.indexOf(subscription)
    if (index !== -1) {
      this.subscriptions[index].unsubscribe()
      this.subscriptions.splice(index, 1)
    }
  }
  
  /**
   * Unsubscribe from all managed subscriptions and clear the list
   */
  unsubscribeAll(): void {
    this.subscriptions.forEach(subscription => {
      if (!subscription.closed) {
        subscription.unsubscribe()
      }
    })
    this.subscriptions = []
  }
  
  /**
   * Get the current count of active subscriptions
   */
  get count(): number {
    return this.subscriptions.length
  }
}