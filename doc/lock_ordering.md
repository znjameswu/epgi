To avoid deadlock, we ban the usage of holding multiple locks at the same time, except the following conditions with the correct lock order:

Lock ordering manually enforced
1. Two locks in the ProviderObject may coexist. Order: inner lock > value lock.


Lock ordering enforced by API design
1. An element snapshot lock with a provider lock. Order: element snapshot lock > provider lock.
2. An element snapshot lock with a secondary root lock. Order: element snapshot lock > secondary root lock.
3. Multiple layer locks with different depths. Order: Higher > lower. 
4. Multiple layer locks from a group of sibling layers. No order. Such pattern only occurred during painting phase and composition phase and is sequenced to avoid deadlocks. External code should never contend with those phases by simultaneously holding locks from sibling layers.