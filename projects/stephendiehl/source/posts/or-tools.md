---
date: "2018-02-01"
tags: ["python", "optimization", "formal", "logic-programming"]
url: "https://www.stephendiehl.com/posts/or-tools/"
---

# Using Google OR-Tools to do Answer Set Programming

Ever tried manually figuring out the perfect schedule for your team or planning delivery routes? Enter Google OR-Tools, a free Swiss Army knife of a toolkit that tackles these headache-inducing optimization challenges. From constraint programming to linear programming and specialized routing solvers, it packs all the tools you need to solve complex puzzles in computer science. And since Google maintains and improves it, you can focus on solving your specific problems rather than reinventing optimization algorithms from scratch.

Here's a simple example where we need to select people for a team while satisfying multiple constraints:

``` python
from ortools.sat.python import cp_model

model = cp_model.CpModel()

persons = ['Tony', 'Bob', 'Ann', 'Carl', 'Amber', 'Peter']
person_vars = {p: model.NewBoolVar(p) for p in persons}

# Select exactly 2 people from group1
group1 = ['Bob', 'Ann', 'Carl', 'Amber', 'Peter']
model.Add(sum(person_vars[p] for p in group1) == 2)

# Select exactly 1 person from group2
group2 = ['Bob', 'Amber']
model.Add(sum(person_vars[p] for p in group2) == 1)

# Amber must be selected
model.Add(person_vars['Amber'] == 1)

# Total team size must be 4
model.Add(sum(person_vars.values()) == 4)

solver = cp_model.CpSolver()
status = solver.Solve(model)

if status == cp_model.OPTIMAL or status == cp_model.FEASIBLE:
    print("Solution found:")
    for person, var in person_vars.items():
        if solver.Value(var):
            print(person)
else:
    print("No solution found.")
```

The other textbook example of this kind of problem is course scheduling:

``` python
from ortools.sat.python import cp_model

def create_course_schedule():
    model = cp_model.CpModel()

    # Parameters
    num_time_slots = 5  # Time slots per day
    num_rooms = 3
    courses = ['Math', 'Physics', 'Chemistry', 'Biology', 'English']
    teachers = ['Smith', 'Johnson', 'Williams']

    # Decision variables
    slots = {}
    for course in courses:
        for time in range(num_time_slots):
            for room in range(num_rooms):
                slots[(course, time, room)] = model.NewBoolVar(f'{course}_{time}_{room}')

    # Each course must be scheduled exactly once
    for course in courses:
        model.Add(sum(slots[(course, t, r)]
                     for t in range(num_time_slots)
                     for r in range(num_rooms)) == 1)

    # Only one course per room per time slot
    for time in range(num_time_slots):
        for room in range(num_rooms):
            model.Add(sum(slots[(c, time, room)] for c in courses) <= 1)

    # Solve the model
    solver = cp_model.CpSolver()
    status = solver.Solve(model)

    if status == cp_model.OPTIMAL or status == cp_model.FEASIBLE:
        print("\nSchedule:")
        for time in range(num_time_slots):
            print(f"\nTime Slot {time + 1}:")
            for room in range(num_rooms):
                for course in courses:
                    if solver.Value(slots[(course, time, room)]):
                        print(f"Room {room + 1}: {course}")
    else:
        print("No solution found.")

create_course_schedule()
```

The distance matrix represents the distances between each pair of locations, where each row and column corresponds to a location (depot or customer). For example, distances\[1\]\[2\] = 6 means it takes 6 units to travel from Customer 1 to Customer 2. The matrix is symmetric, meaning the distance from A to B equals the distance from B to A. The first row/column (index 0) represents the depot, while subsequent indices represent customers.

``` python
from ortools.constraint_solver import routing_enums_pb2
from ortools.constraint_solver import pywrapcp

def create_delivery_route():
    # Distance matrix
    distances = [
        [0, 2, 9, 10],  # Depot
        [2, 0, 6, 8],   # Customer 1
        [9, 6, 0, 3],   # Customer 2
        [10, 8, 3, 0],  # Customer 3
    ]

    manager = pywrapcp.RoutingIndexManager(len(distances), 1, 0)
    routing = pywrapcp.RoutingModel(manager)

    def distance_callback(from_index, to_index):
        from_node = manager.IndexToNode(from_index)
        to_node = manager.IndexToNode(to_index)
        return distances[from_node][to_node]

    transit_callback_index = routing.RegisterTransitCallback(distance_callback)
    routing.SetArcCostEvaluatorOfAllVehicles(transit_callback_index)

    search_parameters = pywrapcp.DefaultRoutingSearchParameters()
    search_parameters.first_solution_strategy = (
        routing_enums_pb2.FirstSolutionStrategy.PATH_CHEAPEST_ARC)

    solution = routing.SolveWithParameters(search_parameters)

    if solution:
        print("Optimal delivery route:")
        index = routing.Start(0)
        route = ['Depot']
        while not routing.IsEnd(index):
            index = solution.Value(routing.NextVar(index))
            if not routing.IsEnd(index):
                route.append(f'Customer {manager.IndexToNode(index)}')
        route.append('Depot')
        print(' -> '.join(route))

create_delivery_route()
```
