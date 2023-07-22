To avoid deadlock, we ban the usage of holding multiple locks at the same time, except the following conditions with the correct lock order:

Lock ordering manually enforced
1. Two locks in the ProviderObject may coexist. Order: inner lock > value lock.


Lock ordering enforced by API design
1. An element snapshot lock with one provider lock. Order: element snapshot lock > provider lock.
2. An element snapshot lock with one secondary root lock. Order: element snapshot lock > secondary root lock.