---
date: "2022-04-28"
tags: ["physics", "python", "quantum mechanics"]
url: "https://www.stephendiehl.com/posts/classical_quantum/"
---

# Simulating Qubits with Python (Classically)

Ever wondered what it would be like to play with a quantum computer? While we can't all have a supercooled quantum processor in our basement (yet), we can do the next best thing—simulate one with regular Python code. Sure, it's hilariously inefficient but surprisingly educational! We're going to build a simple quantum simulator from scratch. It won't scale well (at all), but it'll give us a hands-on feel for qubits, superposition, and look like when simulated in software.

The fundamental unit of quantum information is the **qubit**. Unlike a [classical bit](https://www.smbc-comics.com/comic/the-talk-3), firmly fixed as either 0 or 1, a qubit can exist in a **superposition**, a combination of both states simultaneously. We represent the state of a single qubit, often denoted as `|ψ>`, mathematically like this: `α|0> + β|1>`. Here, `|0>` and `|1>` represent the two fundamental basis states (analogous to classical 0 and 1), while `α` and `β` are complex numbers called **amplitudes**. These amplitudes aren't arbitrary; the squares of their absolute magnitudes, `|α|²` and `|β|²`, give the probability of finding the qubit in the `|0>` state or the `|1>` state, respectively, when we measure it. A core rule is that these probabilities must sum to one: `|α|² + |β|² = 1`.

Things get exponentially more complex when we have multiple qubits. For a system with *n* qubits, there are `2^n` possible classical configurations (like `00`, `01`, `10`, `11` for two qubits). The quantum state of this *n*-qubit system requires a complex amplitude for *each* of these `2^n` basis states. For example, a 3-qubit system (`n=3`) exists in an 8-dimensional complex vector space, with basis states `|000>`, `|001>`, ..., `|111>`. The complete description of the system's state is a list, or **state vector**, containing these `2^n` complex amplitudes.

This exponential growth is the crux of quantum computing's power and also why it's so hard to simulate classically. Doubling the number of qubits squares the number of classical states, demanding exponentially more memory and processing power for simulation. Our simulator effectively manages this state vector to simulate what would be done in hardware on a real quantum computer.

Let's look at the Python implementation, encapsulated in a `QuantumSystem` class.

``` python
import math
import random
import cmath
from typing import List, Tuple, Sequence

SQRT2 = math.sqrt(2.0)

class QuantumSystem:
    def __init__(self, num_qubits: int):
        if num_qubits < 1:
            raise ValueError("Number of qubits must be at least 1.")

        self.num_qubits: int = num_qubits
        self.num_states: int = 1 << num_qubits

        self.amplitudes: List[complex] = [complex(0.0)] * self.num_states
        self.amplitudes[0] = complex(1.0)

    def _validate_qubit_index(self, qubit_index: int):
        if not (0 <= qubit_index < self.num_qubits):
            raise ValueError(
                f"Invalid qubit index: {qubit_index}. "
                f"Must be between 0 and {self.num_qubits - 1}."
            )

    def _validate_qubit_indices(self, *indices: int):
        seen = set()
        for index in indices:
            self._validate_qubit_index(index)
            if index in seen:
                 raise ValueError(f"Duplicate qubit index specified: {index}")
            seen.add(index)

    def measure(self) -> Tuple[int, ...]:
        probabilities: List[float] = [abs(amp) ** 2 for amp in self.amplitudes]

        measured_state_index: int = random.choices(
            population=range(self.num_states), weights=probabilities, k=1
        )[0]

        self.amplitudes = [complex(0.0)] * self.num_states
        self.amplitudes[measured_state_index] = complex(1.0)

        measured_bits = tuple(
            (measured_state_index >> bit_pos) & 1
            for bit_pos in range(self.num_qubits)
        )
        return measured_bits

    def get_probabilities(self) -> List[float]:
        return [abs(amp) ** 2 for amp in self.amplitudes]

    def apply_h(self, target_qubit: int):
        self._validate_qubit_index(target_qubit)

        new_amplitudes = self.amplitudes[:]

        target_mask = 1 << target_qubit

        for i in range(self.num_states // 2):
            lower_mask = target_mask - 1
            upper_mask = ~lower_mask
            i0 = (i & lower_mask) | ((i << 1) & upper_mask)
            i1 = i0 | target_mask

            amp0 = self.amplitudes[i0]
            amp1 = self.amplitudes[i1]

            new_amp0 = (amp0 + amp1) / SQRT2
            new_amp1 = (amp0 - amp1) / SQRT2

            new_amplitudes[i0] = new_amp0
            new_amplitudes[i1] = new_amp1

        self.amplitudes = new_amplitudes

    def apply_cnot(self, control_qubit: int, target_qubit: int):
        self._validate_qubit_indices(control_qubit, target_qubit)

        control_mask = 1 << control_qubit
        target_mask = 1 << target_qubit

        new_amplitudes = self.amplitudes[:]

        for i0 in range(self.num_states):
            if (i0 >> control_qubit) & 1:
                i1 = i0 ^ target_mask
                if i0 < i1:
                    new_amplitudes[i0], new_amplitudes[i1] = \
                        self.amplitudes[i1], self.amplitudes[i0]

        self.amplitudes = new_amplitudes

    def apply_t(self, target_qubit: int):
        self._validate_qubit_index(target_qubit)

        phase_shift = cmath.exp(1j * cmath.pi / 4.0)
        target_mask = 1 << target_qubit

        for i in range(self.num_states):
            if (i >> target_qubit) & 1:
                self.amplitudes[i] *= phase_shift

    def apply_original_pi_over_eight(self, target_qubit: int):
        self._validate_qubit_index(target_qubit)

        phase_zero = cmath.exp(-1j * cmath.pi / 8.0)
        phase_one = cmath.exp(1j * cmath.pi / 8.0)
        target_mask = 1 << target_qubit

        for i in range(self.num_states):
            if (i >> target_qubit) & 1:
                self.amplitudes[i] *= phase_one
            else:
                self.amplitudes[i] *= phase_zero

    def __repr__(self) -> str:
        state_strs = []
        for i, amp in enumerate(self.amplitudes):
            if not cmath.isclose(amp, 0.0):
                basis_state = format(i, f'0{self.num_qubits}b')
                state_strs.append(f"{amp:.3f}|{basis_state}>")
        if not state_strs:
            return "QuantumSystem(num_qubits={}, state=Zero Vector)".format(
                self.num_qubits
            )
        return " + ".join(state_strs)

    def __str__(self) -> str:
        return self.__repr__()
```

When we create a `QuantumSystem` instance, like `system = QuantumSystem(3)`, the `__init__` method sets up the state vector. It checks if the number of qubits is valid, stores it in `self.num_qubits`, and calculates the total number of states (`self.num_states = 2**num_qubits`). The core attribute, `self.amplitudes`, is initialized as a list of `self.num_states` complex zeros. Crucially, it sets the amplitude of the very first state (index 0, corresponding to `|00...0>`) to `complex(1.0)`, representing the standard starting state for quantum computations.

To see the state we're in, the `__repr__` method provides a convenient string format, displaying non-zero amplitudes alongside their corresponding basis states written in binary (e.g., `(0.707+0.000j)|00> + (0.707+0.000j)|11>`).

Quantum computation proceeds by applying **quantum gates** to the qubits. These gates are the quantum analogues of classical logic gates (like NOT, AND, OR). Mathematically, they correspond to **unitary transformations** applied to the state vector. Applying a gate modifies the amplitudes, potentially creating superposition or entanglement. Our class implements several fundamental gates.

The **Hadamard gate**, implemented in `apply_h`, is essential for creating superposition. Applied to a qubit in state `|0>`, it produces `(|0> + |1>) / sqrt(2)`; applied to `|1>`, it yields `(|0> - |1>) / sqrt(2)`. When applied to a specific `target_qubit` in a multi-qubit system, it affects all pairs of basis states that differ *only* in that qubit's position. The code efficiently handles this by looping through `num_states / 2` pairs. For each pair `(i0, i1)` (where `i0` has the target qubit as 0 and `i1` has it as 1), it calculates the new amplitudes `amp0'` and `amp1'` using the Hadamard transformation rules: `amp0' = (amp0 + amp1) / sqrt(2)` and `amp1' = (amp0 - amp1) / sqrt(2)`. Notice how every operation potentially touches *all* amplitudes indirectly, reflecting the holistic nature of the quantum state.

The **Controlled-NOT (CNOT) gate**, implemented in `apply_cnot`, is a two-qubit gate crucial for creating **entanglement** – the spooky connection where qubits remain correlated even when separated. It flips the state of the `target_qubit` if and only if the `control_qubit` is in the `|1>` state. In the state vector representation, this means swapping the amplitudes of pairs of basis states where the control bit is 1, and which differ only in the target bit position. The implementation iterates through the state indices (`i0`). If the control bit in `i0` is 1, it finds the corresponding index `i1` where the target bit is flipped (`i1 = i0 ^ target_mask`). To avoid double-swapping, it only performs the amplitude swap between `self.amplitudes[i0]` and `self.amplitudes[i1]` if `i0 < i1`, ensuring each relevant pair is processed exactly once.

**Phase gates** modify the complex phase of the amplitudes without changing the measurement probabilities. The `apply_t` method implements the standard **T gate**, which applies a phase shift of `exp(i*pi/4)` to the amplitude of any basis state where the `target_qubit` is 1. The `apply_original_pi_over_eight` method implements a slightly different gate from an earlier design, applying `exp(-i*pi/8)` if the qubit is 0 and `exp(i*pi/8)` if it's 1. Both work by iterating through the amplitudes and multiplying by the appropriate complex phase factor if the state index `i` meets the condition related to the `target_qubit`.

Finally, how do we extract a classical result? This is done via **measurement**, implemented in the `measure` method. Quantum measurement is probabilistic. The `measure` method first calculates the probability of measuring each basis state `i` by taking the squared absolute value of its amplitude (`abs(amp)**2`). It then uses Python's `random.choices` to perform a weighted random selection from the possible state indices (`range(self.num_states)`) using the calculated probabilities as weights. This returns the index `measured_state_index` of the single classical state the system collapses into. The simulation then enforces this collapse: the state vector `self.amplitudes` is reset to all zeros, except for the measured state's amplitude, which is set to 1.0. The method returns the outcome as a tuple of classical bits (0s and 1s) derived from the `measured_state_index`.

We can see these components in action through examples. First, let's create a 3-qubit system and apply a Hadamard gate to the second qubit.

``` python
print("--- Example 1: Original Sequence ---")
system1 = QuantumSystem(3)
print(f"Initial state: {system1}")
system1.apply_h(1)
print(f"After H(1):    {system1}")
system1.apply_h(2)
print(f"After H(2):    {system1}")
system1.apply_original_pi_over_eight(1)
print(f"After Pi/8(1): {system1}")
system1.apply_cnot(control_qubit=1, target_qubit=0)
print(f"After CNOT(1,0): {system1}")
print(f"Probabilities: {[f'{p:.3f}' for p in system1.get_probabilities()]}")
measurement_result = system1.measure()
print(f"Measured state: {measurement_result}")
print(f"State after measurement: {system1}")
```

Next, let's prepare the Bell state.

``` python
print("--- Example 2: Bell State Preparation ---")
bell_system = QuantumSystem(2)
print(f"Initial state: {bell_system}")
bell_system.apply_h(0)
print(f"After H(0):    {bell_system}")
bell_system.apply_cnot(control_qubit=0, target_qubit=1)
print(f"After CNOT(0,1): {bell_system}")
print("Measuring Bell state 10 times:")
counts = {}
for _ in range(10):
    bell_system = QuantumSystem(2)
    bell_system.apply_h(0)
    bell_system.apply_cnot(control_qubit=0, target_qubit=1)
    result = bell_system.measure()
    counts[result] = counts.get(result, 0) + 1
print(f"Measurement counts: {counts}")
```

Now, let's prepare the Greenberger-Horne-Zeilinger state.

``` python
print("--- Example 3: GHZ State Preparation ---")
ghz_system = QuantumSystem(3)
print(f"Initial state: {ghz_system}")
ghz_system.apply_h(0)
print(f"After H(0):    {ghz_system}")
ghz_system.apply_cnot(control_qubit=0, target_qubit=1)
print(f"After CNOT(0,1): {ghz_system}")
ghz_system.apply_cnot(control_qubit=0, target_qubit=2)
print(f"After CNOT(0,2): {ghz_system}")
print("Measuring GHZ state 10 times:")
counts_ghz = {}
for _ in range(10):
    ghz_system = QuantumSystem(3)
    ghz_system.apply_h(0)
    ghz_system.apply_cnot(control_qubit=0, target_qubit=1)
    ghz_system.apply_cnot(control_qubit=0, target_qubit=2)
    result = ghz_system.measure()
    counts_ghz[result] = counts_ghz.get(result, 0) + 1
print(f"Measurement counts: {counts_ghz}")
```

These examples demonstrate preparing famous entangled states like the Bell state (`(|00> + |11>)/sqrt(2)`) and the [Greenberger–Horne–Zeilinger (GHZ) state](https://en.wikipedia.org/wiki/Greenberger%E2%80%93Horne%E2%80%93Zeilinger_state) (`(|000> + |111>)/sqrt(2)`). Running the measurement multiple times (after resetting the state for simulation purposes) clearly shows the correlations inherent in entanglement – the outcomes will always be `(0, 0)` or `(1, 1)` for the Bell state, and `(0, 0, 0)` or `(1, 1, 1)` for the GHZ state, with roughly equal probability.

Simulating quantum systems classically using state vectors is really only useful as an educational tool. It forces us to confront the exponential resources required (`2^n` complex numbers, operations affecting all states) so it's not a practical way to simulate even small quantum systems. While this approach quickly becomes infeasible for large numbers of qubits (simulating 50-60 qubits pushes the limits of current supercomputers), it hints at what quantum computers may be capable of ... that is if we can ever build them.
