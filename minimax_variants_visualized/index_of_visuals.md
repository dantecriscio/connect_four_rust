## Base Algorithms on Unsorted Game Tree
| Alpha-Beta-Optimized? | Minimax | Negamax |
| - | - | - |
| No | [00](./unsorted_normal_minimax.md)  | [01](./unsorted_normal_negamax.md)  |
| Yes  | [10](./unsorted_alpha_beta_minimax.md)  | [11](./unsorted_alpha_beta_negamax.md)  |

This table has two dimensions of `0`s and `1`s. The value `0`s denotes a less optimal base algorithm and `1`s denotes a more optimal base algorithm.
Because the alpha-beta optimized version of negamax is the best base algorithm, it will be used exclusively for all other discussions and subsequent algorithms.


## Effect Of Sorting With Heuristics
| Worst-To-Best | Unsorted | Best-To-Worst |
| - | - | - |
| [0](./worst_sorting_alpha_beta_negamax.md) | [1](./unsorted_alpha_beta_negamax.md) (same as `11` in table above) | [2](./best_sorting_alpha_beta_negamax.md) |

In this table, increasing the number means more pruning and better performance.



## Null Window and Its Applications
See [Null Window](./null_window.md).

Some applications of Null Window search are in:
* [NegaScout / Principal Variation Search](./negascout.md)
* [MTD(f)](./mtdf.md)
