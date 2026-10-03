## Example
```mermaid
graph TD
    Root:::MaximizerNode
    Root --> A:::MinimizerNode
        A --> AA:::MaximizerNode
            AA --> AAA:::MinimizerNode
                AAA[1]
            AA --> AAB:::MinimizerNode
                AAB[-1]
            AA{{1}}
        A --> AB:::MaximizerNode
            AB --> ABA:::MinimizerNode
                ABA[4]
            AB --> ABB:::MinimizerNode
                ABB[5]
            AB{{5}}
        A{{1}}
    Root --> B:::MinimizerNode
        B --> BA:::MaximizerNode
            BA --> BAA:::MinimizerNode
                BAA[-4]
            BA --> BAB:::MinimizerNode
                BAB[-3]
            BA{{-3}}
        B --> BB:::MaximizerNode
            BB --> BBA:::MinimizerNode
                BBA[-5]
            BB --> BBB:::MinimizerNode
                BBB[-4]
            BB{{-4}}
        B{{-4}}
    Root --> C:::MinimizerNode
        C --> CA:::MaximizerNode
            CA --> CAA:::MinimizerNode
                CAA[3]
            CA --> CAB:::MinimizerNode
                CAB[0]
            CA{{3}}
        C --> CB:::MaximizerNode
            CB --> CBA:::MinimizerNode
                CBA[4]
            CB --> CBB:::MinimizerNode
                CBB[-3]
            CB{{4}}
        C{{3}}
    Root{{3}}

    %% Nodes
    classDef MaximizerNode fill:pink,color:black;
    classDef MinimizerNode fill:lightBlue,color:black;

    %% Best Path
    linkStyle 14,15,16 stroke:lightGreen;
```

## Legend
```mermaid
graph LR
    Max{{"Maximizer Node"}}:::MaximizerNode
    Min{{"Minimizer Node"}}:::MinimizerNode
    Leaf["Leaf Node (Minimizer's Turn)"]:::MinimizerNode

    Max -->|"Principal Variation"| Min -->|"Non-Principal Variation"| Leaf

    %% Nodes
    classDef MaximizerNode fill:pink,color:black;
    classDef MinimizerNode fill:lightBlue,color:black;

    %% Best Path
    linkStyle 0 stroke:lightGreen;
```

## Notes
The base algorithm for any two-player, turn-based game with no hidden information and no randomness.
It is somewhat intuitive that this algorithm can even serve as a *definition* of what is meant by statements like "with perfect play, this game will end with ...".
All subsequent algorithms have the goal of returning the same principal variation and starting node evaluation, but will differ in their implementations to pursue performance improvements.

It can be expressed with the pseudocode
```
fn minimax<T:Ord>(node: NodeType, is_maximizer: bool) -> T {
    if node.is_leaf() {
        return node.get_leaf_score();
    }

    let child_eval_iter = node.generate_child_moves(is_maximizer)
        .map(|child_node| minimax(child_node, !is_maximizer));

    if is_maximizer {
        child_eval_iter.max()
    } else {
        child_eval_iter.min()
    }
}
```

To return the principal variation, each call can also somehow return or store a reference to its best child.
These can then be stitched together into a path from the root to leaf.
