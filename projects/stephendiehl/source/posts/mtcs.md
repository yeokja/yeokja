---
date: "2024-09-04"
tags: ["ai", "python"]
url: "https://www.stephendiehl.com/posts/mtcs/"
---

# Monte Carlo Tree Search

Monte Carlo Tree Search is a robust algorithm used for decision making in game playing and reinforcement learning. It is a heuristic search algorithm that combines the best aspects of tree search and Monte Carlo simulation. MCTS is particularly useful when dealing with large and complex search spaces, such as in neurosymbolic systems and reinforcement learning.

Today let's implement MCTS in Python. First we need to define a `Node` class that will be used to represent the state.

``` python
rom dataclasses import dataclass, field
from collections import defaultdict
import math
from typing import Dict, Set, List, Optional
from abc import ABC, abstractmethod
import random


class Node(ABC):
    """Abstract base class for game state nodes."""

    @abstractmethod
    def find_children(self) -> Set["Node"]:
        """Return all possible child nodes."""

    @abstractmethod
    def get_random_child(self) -> Optional["Node"]:
        """Return a random child node."""

    @abstractmethod
    def is_terminal(self) -> bool:
        """Check if the node is a terminal state."""

    @abstractmethod
    def reward(self) -> float:
        """Return the reward value for a terminal node."""

    @abstractmethod
    def __hash__(self) -> int:
        """Define a hash function for the node."""

    @abstractmethod
    def __eq__(self, other: object) -> bool:
        """Define equality comparison for nodes."""
```

Now we'll define the `MCTS` class that will hold the search state and perform the tree search.

``` python
@dataclass
class MCTS:
    """Monte Carlo Tree Search implementation."""

    exploration_weight: float = 1.0
    total_rewards: Dict[Node, float] = field(default_factory=lambda: defaultdict(float))
    visit_counts: Dict[Node, int] = field(default_factory=lambda: defaultdict(int))
    children: Dict[Node, Set[Node]] = field(default_factory=lambda: defaultdict(set))
```

Now we'll implement the `select_best_move` method that will select the best move from the current node.

``` python
    def select_best_move(self, node: Node) -> Node:
        """Select the best move from the current node."""
        if node.is_terminal():
            raise ValueError(f"Cannot select move from terminal node: {node}")

        if node not in self.children:
            return node.get_random_child()
```

In the algorithm we'll use the UCT (Upper Confidence Trees) formula to select the best move.

$$UCT(v) = \frac{w_v}{n_v} + c \sqrt{\frac{\ln(N)}{n_v}}$$

Where:

- $w_v$ is the total reward of the node.
- $n_v$ is the number of times the node has been visited.
- $N$ is the total number of simulations.
- $c$ is the exploration constant.

``` python
    def simulate(self, node: Node) -> None:
        """Perform one iteration of the MCTS algorithm."""
        path = self._traverse_tree(node)
        leaf = path[-1]
        self._expand_node(leaf)
        reward = self._simulate_random_playout(leaf)
        self._backpropagate(path, reward)

    def _traverse_tree(self, node: Node) -> List[Node]:
        """Traverse the tree to find an unexplored node."""
        path = []
        while True:
            path.append(node)
            if node not in self.children or not self.children[node]:
                return path
            unexplored = self.children[node] - set(self.children.keys())
            if unexplored:
                path.append(unexplored.pop())
                return path
            node = self._select_uct(node)

    def _expand_node(self, node: Node) -> None:
        """Expand the node by adding its children to the tree."""
        if node in self.children:
            return
        self.children[node] = node.find_children()
```

Now we'll implement the `_simulate_random_playout` method that will simulate a random playout from the given node.

``` python
def _simulate_random_playout(self, node: Node) -> float:
    """Simulate a random playout from the given node."""
    invert_reward = True
    while True:
        if node.is_terminal():
            reward = node.reward()
            return 1 - reward if invert_reward else reward
        node = node.get_random_child()
        invert_reward = not invert_reward
```

Now we'll implement the `_backpropagate` method that will update the statistics for the nodes in the path.

``` python
def _backpropagate(self, path: List[Node], reward: float) -> None:
    """Update the statistics for the nodes in the path."""
    for node in reversed(path):
        self.visit_counts[node] += 1
        self.total_rewards[node] += reward
        reward = 1 - reward
```

The select method will select a child node using the UCT metric.

``` python
    def _select_uct(self, node: Node) -> Node:
        """Select a child node using the UCT formula."""
        assert all(n in self.children for n in self.children[node])
        log_n_parent = math.log(self.visit_counts[node])

        def uct(n: Node) -> float:
            return self.total_rewards[n] / self.visit_counts[
                n
            ] + self.exploration_weight * math.sqrt(log_n_parent / self.visit_counts[n])

        return max(self.children[node], key=uct)
```

And then finally we'll implement the `_calculate_node_score` method that will calculate the score for a node based on its average reward.

``` python
def _calculate_node_score(self, nodeNode) -> float:
    """Calculate the score for a node based on its average reward."""
    if self.visit_counts[node] == 0:
        return float("-inf")
    return self.total_rewards[node] / self.visit_counts[node]
```

Now to use this we need to define a `TicTacToeNode` class that will be used to represent the state of the game.

``` python
class TicTacToeNode(Node):
    def __init__(self, state: str, player: str):
        self.state = state
        self.player = player

    def find_children(self) -> Set["TicTacToeNode"]:
        if self.is_terminal():
            return set()
        return {
            TicTacToeNode(
                self.state[:i] + self.player + self.state[i + 1 :],
                "O" if self.player == "X" else "X",
            )
            for i, value in enumerate(self.state)
            if value == " "
        }

    def get_random_child(self) -> Optional["TicTacToeNode"]:
        if self.is_terminal():
            return None
        empty_spots = [i for i, value in enumerate(self.state) if value == " "]
        index = random.choice(empty_spots)
        return TicTacToeNode(
            self.state[:index] + self.player + self.state[index + 1 :],
            "O" if self.player == "X" else "X",
        )

    def is_terminal(self) -> bool:
        return self.winner() is not None or " " not in self.state

    def reward(self) -> float:
        winner = self.winner()
        if winner is None:
            return 0.5  # Draw
        return 1.0 if winner == self.player else 0.0

    def __hash__(self) -> int:
        return hash(self.state)

    def __eq__(self, other: object) -> bool:
        return isinstance(other, TicTacToeNode) and self.state == other.state

    def winner(self) -> Optional[str]:
        lines = [
            (0, 1, 2),
            (3, 4, 5),
            (6, 7, 8),  # Rows
            (0, 3, 6),
            (1, 4, 7),
            (2, 5, 8),  # Columns
            (0, 4, 8),
            (2, 4, 6),  # Diagonals
        ]
        for line in lines:
            if self.state[line[0]] == self.state[line[1]] == self.state[line[2]] != " ":
                return self.state[line[0]]
        return None
```

Now we can use the `MCTS` class to play a game of TicTacToe.

``` python
def play_game():
    state = " " * 9
    mcts = MCTS()
    board = TicTacToeNode(state, "X")

    print("Initial board:")
    print_board(board.state)

    while True:
        # Human player's turn (O)
        human_move = int(input("Enter your move (0-8): "))
        board = TicTacToeNode(
            board.state[:human_move] + "O" + board.state[human_move + 1 :], "X"
        )
        print("\nBoard after your move:")
        print_board(board.state)

        if board.is_terminal():
            break

        # AI player's turn (X)
        for _ in range(1000):  # Number of MCTS iterations
            mcts.simulate(board)
        board = mcts.select_best_move(board)
        print("\nBoard after AI move:")
        print_board(board.state)

        if board.is_terminal():
            break

    winner = board.winner()
    if winner:
        print(f"\nPlayer {winner} wins!")
    else:
        print("\nIt's a draw!")


def print_board(state: str):
    for i in range(0, 9, 3):
        print(" ".join(state[i : i + 3]))


if __name__ == "__main__":
    play_game()
```
