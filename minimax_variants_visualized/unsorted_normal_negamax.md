## Example
```mermaid
graph TD
    Root:::Node
    Root --> A:::Node
        A --> AA:::Node
            AA --> AAA:::Node
                AAA[-1]
            AA --> AAB:::Node
                AAB[1]
            AA{{1}}
        A --> AB:::Node
            AB --> ABA:::Node
                ABA[-4]
            AB --> ABB:::Node
                ABB[-5]
            AB{{5}}
        A{{-1}}
    Root --> B:::Node
        B --> BA:::Node
            BA --> BAA:::Node
                BAA[4]
            BA --> BAB:::Node
                BAB[3]
            BA{{-3}}
        B --> BB:::Node
            BB --> BBA:::Node
                BBA[5]
            BB --> BBB:::Node
                BBB[4]
            BB{{-4}}
        B{{4}}
    Root --> C:::Node
        C --> CA:::Node
            CA --> CAA:::Node
                CAA[-3]
            CA --> CAB:::Node
                CAB[0]
            CA{{3}}
        C --> CB:::Node
            CB --> CBA:::Node
                CBA[-4]
            CB --> CBB:::Node
                CBB[3]
            CB{{4}}
        C{{-3}}
    Root{{3}}

    %% Nodes
    classDef Node fill:lightGray,color:black;

    %% Best Path
    linkStyle 14,15,16 stroke:lightGreen;
```

## Legend
```mermaid
graph LR
    Parent{{"Parent Node"}}:::Node
    Leaf["Leaf Node"]:::Node

    Parent -->|"Principal Variation"| Leaf
    Parent -->|"Non-Principal Variation"| Leaf

    %% Nodes
    classDef Node fill:lightGray,color:black;

    %% Best Path
    linkStyle 0 stroke:lightGreen;
```

## Notes
A slight simplification and performance improvement over the [base minimax algorithm](./unsorted_normal_minimax.md).

It can be expressed with the pseudocode
```
fn negamax(node: NodeType) -> ReturnEvalType {
    if node.is_leaf() {
        node.get_leaf_score()
    } else {
        node.children_iter()
            .map(|child_node| -negamax(child_node.flip_game_state()))
            .max()
    }
}
```

At each step of the evaluation process, the algorithm believes that it is the maximizer / player-1.
It then "lies" to its children, letting them think that they are the maximizer / player-1.
It does this by "flipping the game state"; in tic-tak-toe this would involve replacing all the "X's with O's" and vice-versa.

Each recursive call will then naturally generate a response saying how good the child position is for the other player.
These are immediately negated, so that all values say how good the position is for the current player, and then the max is returned.

It is simpler in the sense that a recursive DFS implementation of the `base minimax algorithm` would have a boolean (`is_maximizer`) which must be:
* Checked before generating the possible child positions (which moves are legal)
* Checked before aggregating the child evaluations (maximize or minimize)
* Negated before being passed in the recursive function call

These `if-else` statements and negations are removed completely with this algorithm.
The only added complexity comes from needing to flip the entire game state. However, this is generally easy and fast in practice.
So this algorithm generally leads to less code and better performance.

