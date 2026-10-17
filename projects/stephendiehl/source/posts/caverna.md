---
date: "2026-02-21"
tags: ["formal-methods", "humor", "game-theory", "lean", "formal"]
url: "https://www.stephendiehl.com/posts/caverna/"
---

# Optimal Caverna Gameplay via Formal Methods

I always win at Caverna (Uwe Rosenberg's [classic European worker placement](https://boardgamegeek.com/boardgame/102794/caverna-the-cave-farmers) tabletop board game). Always. But "always" just means "every time so far," and I needed something with more mathematical permanence. So I formalized the entire game in Lean 4 and proved that my strategy is the unique weakly dominant pure strategy across every possible game configuration. My friends think this is excessive. My friends also lose at Caverna. Unrelated, I don't get invited to board game night much anymore.

Caverna: The Cave Farmers is the 2013 sequel to [Agricola](https://boardgamegeek.com/boardgame/31260/agricola), a game about feeding dwarfs who live in caves and do a suspicious amount of farming. You place workers, gather resources, breed animals, excavate caverns, furnish rooms, and at the end of 12 rounds the scoring formula totals up everything you've accomplished and everything you failed to accomplish. It's a good game. It's also, in the 2-player variant, a finite deterministic perfect-information system with discrete phases, which means it's a labeled transition system, which means it's amenable to formal verification. So I did that.

There is a very small mathematical model here. We just define two things: an `init` predicate describing the starting state, and a `trans` relation describing every legal state-to-state move. Then instead of running the game, you prove properties about it by structural induction: show the property holds at the start, show every legal transition preserves it, and you've covered every possible game without enumerating a single one. It's the same technique used in hardware verification and protocol proofs, applied to a board game.

The project is about 3,000 lines of Lean 4 spread across 19 modules: 11 definition files modeling the complete game (all 24 action spaces, all 48 unique furnishing tiles, the expedition loot system, board geometry, harvest schedule, scoring formula) and 8 theorem files containing 176 machine-checked proofs. The model covers all 2,880 possible 2-player game setups (144 card orderings times 20 harvest marker placements).

The main result is that **furnishing rush is the weakly dominant strategy**. It is the optimal response to every opponent, in every setup, regardless of which cards come out when or where the harvest markers land.

The [interactive proof blueprint](https://sdiehl.github.io/caverna-formal/blueprint/) has the full derivation with a dependency graph showing how every theorem connects. The [source code](https://github.com/sdiehl/caverna-formal) compiles.

## Labeled Transition Systems

A labeled transition system (LTS) is a triple $(S, A, T)$ where $S$ is a set of states, $A$ is a set of actions, and $T \subseteq S \times A \times S$ is a transition relation specifying which state changes are legal. You start in some initial state satisfying an `init` predicate. The system evolves by taking actions: if $(s, a, s') \in T$, you can move from state $s$ to state $s'$ by performing action $a$. A state is reachable if there's a finite chain of transitions from an initial state to it. A property is an invariant if it holds on every reachable state.

In Lean 4, the LTS is three fields:

```
structure LTS (State : Type) (Action : Type) where
  init : State -> Prop
  trans : State -> Action -> State -> Prop
```

Reachability is an inductive type with two constructors: initial states are reachable, and if you can reach $s$ and take action $a$ to get to $s'$, then $s'$ is reachable:

```
inductive Reachable (sys : LTS State Action) : State -> Prop where
  | init : forall s, sys.init s -> Reachable sys s
  | step : forall s a s', Reachable sys s -> sys.trans s a s' -> Reachable sys s'
```

In plain terms: the LTS is the rulebook encoded as a state transition relation. `init` says what the starting board looks like, and `trans` encodes every legal move as a state-to-state constraint. The entire Caverna rulebook fits in about 250 lines of Lean: a 160-line `applyPlacement` function matching on all 24 action spaces with their effects, and a 90-line `cavernaLTS` definition wiring up phase transitions, harvest logic, and round progression. It's a relation rather than a function because Caverna has sub-choices within actions (sow grain vs. sow vegetable, which furnishing tile to install, which loot to take), so a single action space placement can lead to multiple successor states. The relation captures all of them.

Once you have `init` and `trans`, you prove a property holds on all reachable states without enumerating them. You prove the base case (holds on initial states) and the inductive step (every valid transition preserves it). This is strictly stronger than testing. Testing checks specific play sequences. An invariant proof covers every sequence of legal moves across every setup, including sequences no human would ever play.

Here's a small fragment of the Caverna LTS to give the flavor of how a round of play works as a state machine:

![Fragment of the game state machine for one round of play](https://www.stephendiehl.com/images/lts.svg)

Each state records the player's resources and remaining dwarfs to place. Transitions correspond to action space selections. With 13 initial action spaces and 2 dwarfs, even a single round produces $13 \times 12 = 156$ placement sequences. Over 12 rounds with family growth, the branching factor is astronomical, but the LTS doesn't care. The structure is finite, and every path through it is covered by the invariant proofs.

## The Game as a Transition Relation

Board games are natural LTS candidates. In Caverna 2-player, the states are the complete game configurations (round number, phase, both players' inventories, board layouts, available action spaces), the actions are dwarf placements and harvest events, and the transition relation encodes the rulebook.

The game has five phases that cycle within each round:

```
inductive Phase where
  | placeP1      -- player 1 places a dwarf
  | placeP2      -- player 2 places a dwarf
  | harvest      -- harvest phase (feeding, breeding, fields)
  | roundEnd     -- round cleanup, advance to next round
  | gameOver     -- game has ended
```

The full game state tracks everything both players could possibly have, plus the global state of the board:

```
structure GState where
  round : Nat
  phase : Phase
  p1 : FullPlayer
  p2 : FullPlayer
  p1IsFirst : Bool
  placementsLeft : Nat
  acc : AccState
  occupiedSpaces : List ActionSpaceId := []
  harvestSchedule : Nat -> HarvestEvent
  wishIsUrgent : Bool := false
```

The transition relation is a single function with a `match` on `(gs.phase, act)`. Each case is a game rule. Here's the core of the placement logic for Player 1:

```
def cavernaLTS (schedule : Nat -> HarvestEvent) :
    TransitionSystem.LTS GState GameAction where
  init := fun gs => gs = initFullGState schedule
  trans := fun gs act gs' =>
    match gs.phase, act with
    | .placeP1, .place space choice =>
      spaceAvailable gs.round space = true /\
      spaceUnoccupied gs space = true /\
      gs.placementsLeft > 0 /\
      (let (p1', acc') := applyPlacement gs.p1 gs space choice
       let newPlacements := gs.placementsLeft - 1
       let newOccupied := space :: gs.occupiedSpaces
       if newPlacements == 0 then
         gs' = { gs with p1 := p1', acc := acc',
                         phase := .harvest, placementsLeft := 0,
                         occupiedSpaces := newOccupied }
       else
         gs' = { gs with p1 := p1', acc := acc',
                         phase := .placeP2,
                         placementsLeft := newPlacements,
                         occupiedSpaces := newOccupied })
    -- ... Player 2, harvest, round end, game over ...
    | _, _ => False
```

That last line is beautiful. `| _, _ => False` says: any action not explicitly listed is illegal. The transition relation is closed. No undefined behavior, no edge cases, no "the rules don't say I can't." If it's not in the match, it doesn't happen.

The `applyPlacement` function is a 250-line match on all 24 action spaces, each encoding the exact effect from the rulebook. Blacksmithing forges a weapon from ore and runs an expedition. Wish for Children can only grow your family if you have a dwelling with capacity. Excavation gives you stone and lets you carve out cavern/tunnel pairs. Every sub-choice (sow grain vs. sow vegetable, build small pasture vs. large pasture, which furnishing tile to install) is a branch in the `ActionChoice` type.

## The Game Timeline

![Game timeline for 2-player Caverna](https://www.stephendiehl.com/images/timeline.svg)

Twelve rounds. Green nodes are harvest rounds where dwarfs must be fed. Two critical milestones: "Wish for Children" at round 4 enables the first family growth (from 2 dwarfs to 3), and "Family Life" at round 8 enables the second (3 to 5, eventually). One new action space reveals each round, growing from 13 to 24 available choices. The interaction between when cards flip and when harvests hit is the clock that drives the entire strategic analysis.

The timeline matters because of this: without growth you get 44 total dwarf placements across all 12 rounds. With one growth at round 4, you get 47. With both growths, you get 56. That's a 27% increase in total actions from growing your family as fast as possible, and since actions are the binding constraint on everything else (scoring, food, resources), the 12-placement gap between "no growth" and "both growths" is the single most important strategic lever in the game.

## Refinement Types: Making Illegal States Unrepresentable

One of the most satisfying patterns in the formalization is using dependent types to make illegal game states impossible to construct. Weapons are the clearest example. In the physical game, weapon strength ranges from 1 to 14. You could model this as a bare `Nat` and hope nobody passes in 0 or 15. Or you could make the type system enforce the constraint:

```
structure Weapon where
  strength : Nat
  h__min : strength >= 1
  h__max : strength <= 14
```

Every `Weapon` value carries proof that its strength is in range. This means `forgeWeapon` must produce evidence that the forged strength is valid, and `upgradeWeapon` must show that incrementing stays within bounds:

```
def forgeWeapon (oreSpent : Nat) (h__pos : oreSpent >= 1) : Option Weapon :=
  if h : oreSpent <= maxInitialWeaponStrength then
    some { strength := oreSpent
         , h__min := h__pos
         , h__max := by simp [maxInitialWeaponStrength] at h; omega }
  else
    none

def upgradeWeapon (w : Weapon) : Weapon :=
  if h : w.strength < maxWeaponStrength then
    { strength := w.strength + 1
    , h__min := by omega
    , h__max := by simp [maxWeaponStrength] at h; omega }
  else
    w
```

The `by omega` calls are Lean's linear arithmetic tactic closing the proof obligations automatically. If `w.strength < 14`, then `w.strength + 1 <= 14`. If `oreSpent >= 1`, then `strength >= 1`. The type checker verifies this at compile time. No weapon in the entire formalization can ever have strength 0 or 15.

The same pattern shows up in `RoundPlacements`, which carries proof that all four dwarf placements within a round go to distinct action spaces:

```
structure RoundPlacements where
  firstPlayer1  : ActionSpaceId
  secondPlayer1 : ActionSpaceId
  firstPlayer2  : ActionSpaceId
  secondPlayer2 : ActionSpaceId
  h__distinct12 : firstPlayer1 != secondPlayer1
  h__distinct13 : firstPlayer1 != firstPlayer2
  h__distinct14 : firstPlayer1 != secondPlayer2
  h__distinct23 : secondPlayer1 != firstPlayer2
  h__distinct24 : secondPlayer1 != secondPlayer2
  h__distinct34 : firstPlayer2 != secondPlayer2
```

Six distinctness proofs, one for each pair. You literally cannot construct a `RoundPlacements` where two dwarfs share an action space. The game rule is baked into the type.

## The Weapon System

![Weapon strength growth path](https://www.stephendiehl.com/images/weapon_path.svg)

Weapons are forged at strength 1 through 8 (costing that many ore), then grow by +1 per expedition, capping at 14. The blue range is forgeable; the peach range requires expedition grinding. Cattle loot unlocks at strength 9, which means even if you forge at max (8 ore), you still need at least one expedition before you can get cattle.

The loot table is an inductive type with minimum strength requirements for each item:

```
def LootItem.minStrength : LootItem -> Nat
  | .allWeaponsPlus1    => 1
  | .dog                => 1
  | .wood               => 1
  | .grain              => 2
  | .sheep              => 2
  | .stone              => 3
  | .donkey             => 3
  | .ore                => 4
  | .wildBoar           => 4
  | .stableFree         => 5
  | .gold2              => 6
  | .furnishCavern      => 7
  | .buildFencesCheap   => 8
  | .cattle             => 9
  | .dwelling           => 10
  | .sow                => 11
  | .breedTwoTypes      => 12
  | .furnishCavernAgain => 14
```

At strength 1 you can loot a dog or a stick. At strength 14 you can furnish a second cavern for free. The loot count at key strengths: 3 at strength 1, 13 at strength 8, 18 at strength 14. That's 5 premium items (cattle, dwelling, sow, breed, second furnish) locked behind the expedition grind. Whether the ore investment is worth it is the core question of the weapon rush archetype, and the answer turns out to be: no, not quite.

## The Universal Food Crisis

Every strategy in Caverna must solve the same problem before anything else. Both players face a food deficit at the first harvest:

![Food conversion network](https://www.stephendiehl.com/images/food_flow.svg)

Player 1 starts with 1 food and needs 4 (2 dwarfs times 2 food each). Player 2 starts with 2 food and needs 4. The gaps are 3 and 2 respectively. This is proven as `universal__food__crisis`:

```
theorem universal_food_crisis :
    feedingCost 2 0 - startingFoodP1 = 3 /\
    feedingCost 2 0 - startingFoodP2 = 2 := by decide
```

The implication (`food__crisis__shapes__all__strategies`) is that every viable archetype must spend its first few actions on food acquisition. There's no "skip feeding and go straight to scoring" option. The begging marker penalty is $-3$ points each, and the theorem `absolute__floor__is__neg55` shows that a player who takes zero actions across all rounds scores $-55$. The food crisis isn't optional; it's structural.

The food conversion network itself is a delightful mess of exchange rates. Cattle gives 4 food per animal, wild boar gives 3, sheep gives 2, grain gives 1, vegetables give 2, gold converts lossily at $n-1$ (1 gold is wasted as overhead), and rubies are emergency food at 2+ each. And then there are donkeys, which have a superlinear pairing bonus that I spent an embarrassing amount of time formalizing:

```
def donkeyFoodValue (n : Nat) : Nat :=
  let pairs := n / 2
  let remainder := n % 2
  pairs * 3 + remainder * 1

theorem donkey__superlinear :
    donkeyFoodValue 2 > donkeyFoodValue 1 + donkeyFoodValue 1 := by decide
```

Two donkeys together yield 3 food, but individually they'd give 1 + 1 = 2. The whole is greater than the sum of its parts. Uwe Rosenberg almost certainly didn't think about this as a super-additivity property of a set function, but that's what it is, and Lean can prove it.

## The Feeding Cascade

The feeding function is the core survival mechanic. When harvest hits, each dwarf eats 2 food (offspring eat 1). If you don't have enough food, the deficit cascades through your resources: try food first, then convert grain (1 food each), then convert vegetables (2 food each), then take begging markers for anything remaining.

```
def FullPlayer.feed (p : FullPlayer) : FullPlayer :=
  let cost := p.dwarfs * 2 + p.offspring * 1
  if p.food >= cost then
    { p with food := p.food - cost }
  else
    let deficit := cost - p.food
    let p' := { p with food := 0 }
    let grainUsed := min p'.grain deficit
    let p'' := { p' with grain := p'.grain - grainUsed }
    let deficit' := deficit - grainUsed
    let vegUsed := min p''.vegetables (deficit' / 2 + deficit' % 2)
    let vegFood := min (vegUsed * 2) deficit'
    let p''' := { p'' with vegetables := p''.vegetables - vegUsed }
    let deficit'' := deficit' - vegFood
    { p''' with beggingMarkers := p'''.beggingMarkers + deficit'' }
```

And a normal harvest is the composition of three phases in one elegant pipeline:

```
def FullPlayer.normalHarvest (p : FullPlayer) : FullPlayer :=
  p.fieldPhase.feed.breedingPhase
```

Field phase harvests your sown crops. Feed pays the food cost (or generates begging markers). Breeding adds one animal per type that has at least two. The order matters: fields produce food before feeding, and breeding happens after, so newborn animals don't need to be fed in the same round they appear. This ordering rule from page 6 of the Caverna rulebook is encoded in the function composition.

## Furnishing Tiles and the BonusContext

There are 48 furnishing tiles in the game, each with resource costs, base victory points, and (for some) end-game bonus scoring formulas that depend on your final board state. The `BonusContext` struct captures everything a furnishing tile might look at when computing its bonus:

```
structure BonusContext where
  stoneInSupply        : Nat := 0
  oreInSupply          : Nat := 0
  sheepCount           : Nat := 0
  cattleCount          : Nat := 0
  numAdjacentDwellings : Nat := 0
  numArmedDwarfs       : Nat := 0
  numDwarfs            : Nat := 0
  rubyCount            : Nat := 0
  grainCount           : Nat := 0
  vegCount             : Nat := 0
  farmAnimalCount      : Nat := 0
  hasAnyWeapon         : Bool := false
  allDwarfsArmed       : Bool := false
  numYellowTagTiles    : Nat := 0
```

The bonus point function is a match on tile ID that implements every scoring formula from the Caverna appendix:

```
def furnishingBonusPoints (fid : FurnishingId) (ctx : BonusContext) : Nat :=
  match fid with
  | .stoneStorage   => ctx.stoneInSupply
  | .oreStorage     => ctx.oreInSupply / 2
  | .weavingParlor  => ctx.sheepCount / 2
  | .milkingParlor  => ctx.cattleCount
  | .stateParlor    => ctx.numAdjacentDwellings * 4
  | .mainStorage    => ctx.numYellowTagTiles * 2
  | .weaponStorage  => ctx.numArmedDwarfs * 3
  | .suppliesStorage => if ctx.allDwarfsArmed then 8 else 0
  | .broomChamber   => if ctx.numDwarfs >= 6 then 10
                        else if ctx.numDwarfs >= 5 then 5
                        else 0
  | .prayerChamber  => if ctx.hasAnyWeapon then 0 else 8
  | _ => 0
```

The Prayer Chamber is particularly nasty. It gives you 8 free points, but only if none of your dwarfs have weapons. The moment you forge a single weapon, it drops to zero. This creates a genuine strategic dilemma, formalized and proved:

```
theorem prayer__chamber__vs__weapons :
    furnishingBonusPoints .prayerChamber {} = 8 /\
    furnishingBonusPoints .prayerChamber { hasAnyWeapon := true } = 0 :=
  by constructor <;> native_decide
```

The furnishing rush archetype gets its power from stacking compatible bonus tiles: Office Room overhangs, State Parlor for +4 per adjacent dwelling (up to +16), Broom Chamber for +5 or +10 based on dwarf count, Prayer Chamber for +8 (no weapons). These bonuses compound. A player with 5 dwarfs, 4 adjacent dwellings, and no weapons can pull +39 bonus points from three tiles. That's why furnishing rush has the highest ceiling.

## The Strategy Space

With the LTS, scoring function, and food economy in place, the next question is: what are the high-level plans a player can actually follow? The transition relation defines roughly $10^{30}$ possible play sequences, but the overwhelming majority of them are nonsense. Consider a player who forges a weapon in round 1 (spending precious ore), then never goes on an expedition, sows a field in round 3 but never harvests grain from it, builds a pasture in round 5 but never acquires an animal, and spends round 8 taking Starting Player for no reason. This player ends the game with a weapon they never used, an empty pasture, a field of rotting grain, and a final score somewhere around 20 points after begging penalties. The LTS covers this path. The invariant proofs hold over it. It is a legal play sequence, and it is also exactly the kind of play sequence that a five-year-old would produce by picking action spaces at random.

The vast majority of the state space looks like this: diffuse, uncommitted paths where the player dabbles in everything and commits to nothing, hemorrhaging tempo and food while accumulating resources that never convert to points. Only a handful of coherent "channels" through the action space graph lead to competitive scores against a rational opponent. A strategy archetype is a consistent pattern of action space usage across the full 12-round game. I identified eight by examining which action spaces and furnishing tiles naturally cluster together: if two actions compete for the same resources or unlock the same scoring categories, they belong to the same archetype. For example, Blacksmithing, Expedition, and weapon-dependent loot all form the weapon rush cluster, while Excavation, Housework, and bonus-scoring furnishings form the furnishing rush cluster. The classification was manual, but the Lean proofs validate it: every archetype's score estimates are derived from the formalized game rules, and the dominance relations are machine-checked over the resulting payoff matrix.

```
inductive StrategyArchetype where
  | furnishingRush
  | weaponRush
  | animalHusbandry
  | miningHeavy
  | balanced
  | peacefulFarming
  | rubyEconomy
  | peacefulCaveEngine
```

But why eight? The answer comes from the structure of the game's scoring channels. A scoring channel is a resource-to-points pathway through the action spaces: the furnishing channel runs through Excavation and Housework into furnished caverns, the weapon channel runs through Blacksmithing into expeditions and loot, and so on. There are exactly six:

```
inductive ScoringChannel where
  | furnishing        -- Excavation + Housework -> caverns -> furnishing tiles
  | weaponExpedition  -- Blacksmithing + Adventure -> weapons -> loot
  | agriculture       -- Clearing + Sustenance -> fields -> crops
  | animalBreeding    -- Sheep Farming + Donkey Farming -> pastures -> animals
  | mining            -- Ore/Ruby Mine Construction -> mines
  | economy           -- Starting Player + Ore Trading + Ruby Mining -> gold/rubies
```

Each archetype commits to a primary channel, and the eight archetypes cover all six channels (`all__channels__covered`). Some channels conflict over shared resources: weapon/expedition and mining both need ore, furnishing and mining both need stone. The `channelsConflict` function encodes these tensions. With 47 productive actions per game and each channel needing at least 6 dedicated actions to reach competitive scoring, a player can invest in at most 2 channels seriously (`budget__bounds__channels`). The archetypes are the maximal compatible channel combinations: 6 single-primary archetypes plus 2 hybrids (balanced spreads across 3 channels, peaceful cave engine uses weapons for food rather than scoring).

The exhaustivity proof (`archetype__channel__surjection`) shows every channel has at least one archetype as its primary. If you're going to score points, you have to go through a channel, and every channel is represented. No viable strategy falls outside the classification.

Each archetype has an estimated scoring ceiling (best case across all setups) and floor (worst case):

![Score estimate ranges for the eight strategy archetypes](https://www.stephendiehl.com/images/score_intervals.svg)

Furnishing rush tops the ceiling at 140 and ties for the highest floor at 60. Weapon rush matches the floor but caps at 120. The others trail off. The interval chart makes the dominance relationships immediately visible: furnishing rush's bar extends further right (higher ceiling) than any other archetype, and its left endpoint (floor) is as good as anyone's.

## The Dominance Hierarchy

Before building the payoff matrix, we can establish partial dominance from the score estimates alone. If strategy $s$ has both a higher ceiling and a higher floor than strategy $t$ (with at least one strict inequality), then $s$ dominates $t$ in score estimates.

![Dominance partial order over strategy archetypes](https://www.stephendiehl.com/images/hasse.svg)

Solid green arrows show dominance via score estimates (higher ceiling and floor). Dashed green arrows indicate dominance established only through the full payoff matrix. Animal Husbandry and Mining Heavy are incomparable in score estimates (one has a higher ceiling, the other a higher floor), but both are weakly dominated by Furnishing Rush in the payoff matrix.

The incomparability is proven constructively:

```
theorem incomparable__strategies__exist :
    not (dominates .miningHeavy .animalHusbandry) /\
    not (dominates .animalHusbandry .miningHeavy) := by decide
```

## The Payoff Matrix

The next question is what happens when two players with potentially different archetypes collide over the shared action spaces. You model this as an $8 \times 8$ payoff matrix $M$ where $M_{ij}$ is the estimated score for the row player when row plays archetype $i$ and column plays archetype $j$.

![The 8x8 payoff matrix](https://www.stephendiehl.com/images/payoff_matrix.svg)

The green border marks the Furnishing Rush row (weakly dominant). Blue borders highlight the diagonal (mirror matchups, always depressed). Cell color intensity reflects payoff magnitude: light green (55) through coral (135).

In Lean, the matrix is a function from `Fin 8 -> Fin 8 -> Int` with all 64 entries hardcoded from the strategy analysis:

```
def payoffMatrix : Fin 8 -> Fin 8 -> Int
  | 0, 0 =>  85 | 0, 1 => 130 | 0, 2 => 135 | 0, 3 => 130
  | 0, 4 => 125 | 0, 5 => 135 | 0, 6 => 135 | 0, 7 => 130
  | 1, 0 =>  80 | 1, 1 =>  75 | 1, 2 => 100 | 1, 3 =>  85
  | 1, 4 =>  95 | 1, 5 => 105 | 1, 6 => 100 | 1, 7 =>  85
  -- ... 48 more entries ...
```

A word on what "estimated" means here. The matrix entries are derived from the formalized game rules (scoring function, food costs, action budgets), not from Monte Carlo simulation or guesswork. But they are estimates in the sense that I haven't exhaustively solved every possible 47-move sequence within each archetype. How sensitive is the result? The `min__dominance__margin__is__5` theorem proves that the minimum gap between the furnishing rush row and the next-best entry in each column is exactly 5 points (occurring in the mirror matchup column). Perturbing any single entry by less than 5 points cannot flip the dominance relation. Even a 10-point swing in a single cell would only affect one column, not the global result. The Nash equilibrium is more sensitive (it depends on the diagonal), but the mirror matchup at 85 would need to drop below the next-best response value of 80 before the equilibrium shifts. The downstream proofs are machine-checked over the matrix as stated, so the formal guarantees are exact conditional on these entries.

We can do better than conditional. Replace each scalar entry with a closed interval bounding the true payoff. Mirror matchups get tight bounds (the point estimate $\pm 2$, since both players execute the same plan and contention is symmetric). Cross-archetype matchups get wider bounds ($\pm 5$, reflecting uncertainty in cross-strategy interaction). The interval payoff matrix looks like this:

```
structure PayoffInterval where
  lo : Int
  hi : Int
  valid : lo <= hi

def intervalPayoff : Fin 8 -> Fin 8 -> PayoffInterval
  | 0, 0 => { lo := 83, hi := 87, valid := by omega }   -- mirror: eps=2
  | 0, 1 => { lo := 125, hi := 135, valid := by omega }  -- vs weapon: eps=5
  -- ... 62 more entries ...
```

Robust weak dominance asks: for every column and every non-furnishing alternative, does the furnishing rush *lower bound* meet or exceed the alternative's *upper bound*? If yes, then furnishing rush dominates for ALL true payoff matrices within these intervals, not just the point estimates.

```
theorem robust_weak_dominance :
    forall (col : Fin 8) (alt : Fin 8), alt != 0 ->
      (intervalPayoff 0 col).lo >= (intervalPayoff alt col).hi := by decide
```

It passes. The tightest cell is column 0 (mirror matchup): furnishing rush $[83, 87]$ vs. weapon rush $[77, 83]$. The margin is $83 - 83 = 0$, a weak tie at the boundary. All other columns have margins of 20 or more (`non__mirror__columns__robust`). If the error bounds were widened by just 1 point, column 0 would fail (`fragility__column__0`), which tells us exactly where the result is most fragile and what the tolerance is.

The interval analysis also gives us robust welfare bounds. Nash welfare lies in $[166, 174]$ and the social optimum in $[200, 220]$. Even in the best case for selfish play and worst case for cooperation, the social optimum still exceeds Nash welfare (`robust__price__of__anarchy`). The prisoner's dilemma structure is not an artifact of the point estimates.

The first row is at least as large as every other row in every column. That's it. That's the whole result. In game theory this property is called weak dominance: a strategy $\sigma^*$ is weakly dominant if for all opponent strategies $o$ and all alternative strategies $\sigma$,

$$M(\sigma^*, o) \geq M(\sigma, o)$$

The Lean proof is `furnishing__rush__weakly__dominant`, and it says exactly this:

```
theorem furnishing_rush_weakly_dominant :
    forall (row : Fin 8) (col : Fin 8),
      payoffMatrix 0 col >= payoffMatrix row col := by decide
```

For all opponents, for all alternatives, the furnishing rush payoff is greater than or equal. The proof is `decide`: Lean's kernel checks all 64 cells. Nobody has to trust my arithmetic.

## The Contention Effect

The diagonal of the payoff matrix is always depressed relative to the off-diagonal entries:

```
theorem diagonal__always__depressed :
    forall (i : Fin 8), exists (j : Fin 8), j /= i /\
      payoffMatrix i j > payoffMatrix i i := by decide
```

Every mirror matchup scores below at least one non-mirror matchup in the same row. This is the contention effect: when both players pursue the same archetype, they fight over the same action spaces. Furnishing rush mirrors compete for excavation and housework. Weapon rush mirrors fight over blacksmithing. Mining mirrors clash on ore mine construction.

The game punishes sameness. It's just that the punishment for sameness (85) is still less painful than the punishment for picking something worse (60 to 80 against an opponent who picked furnishing rush).

## Nash Equilibrium and the Prisoner's Dilemma

From weak dominance, the game theory falls out like dominoes. The best response function $\text{BR} : \text{Strategies} \to \text{Strategies}$ maps each opponent strategy to the row-maximizer in the corresponding column. Since furnishing rush achieves the column maximum everywhere, BR is the constant function:

$$\text{BR}(x) = \text{FurnishingRush} \quad \forall x$$

A Nash equilibrium is a fixed point of the joint best-response correspondence: a pair $(a, b)$ where $a = \text{BR}(b)$ and $b = \text{BR}(a)$. Since BR is constant, the only fixed point is (FurnishingRush, FurnishingRush). Existence and uniqueness in three lines:

```
theorem exactly__one__nash__equilibrium :
    (exists a b, isNashEquilibrium a b) /\
    (forall a b, isNashEquilibrium a b ->
      a = .furnishingRush /\ b = .furnishingRush) :=
  <<.furnishingRush, .furnishingRush, furnishing_mirror_is_nash>,
   furnishing_mirror_unique_nash>
```

The uniqueness proof works by case-splitting on all 64 strategy pairs and observing that only one satisfies the Nash condition:

```
theorem furnishing__mirror__unique__nash :
    forall a b : StrategyArchetype,
      isNashEquilibrium a b ->
        a = .furnishingRush /\ b = .furnishingRush := by
  intro a b h
  have h1 := h.1; have h2 := h.2
  cases a <;> cases b <;> simp_all [bestResponse, isNashEquilibrium]
```

There is exactly one pure Nash equilibrium and it is the one where both players do the same thing. Which is a little tragic.

## The Price of Anarchy

Both players score 85 in the Nash equilibrium. If they could somehow coordinate on different strategies (say furnishing rush versus animal husbandry), the combined welfare would be $135 + 75 = 210$ instead of $85 + 85 = 170$.

![Nash welfare vs. social optimum](https://www.stephendiehl.com/images/welfare.svg)

The price of anarchy is the ratio of the social optimum to the Nash welfare:

$$\text{PoA} = \frac{210}{170} = \frac{21}{17} \approx 1.24$$

Selfish play costs about 19% of the social optimum. The Lean proof is `decide`, because it's just arithmetic:

```
theorem price__of__anarchy__ratio :
    socialOptimumValue = 210 /\ nashWelfare = 170 /\
    210 * 17 = 170 * 21 := by decide
```

The depressing implication is that both players would prefer the other person to play something different, but neither can unilaterally deviate without making themselves worse off. You're stuck at 85 each, staring across the table at someone who also read this blog post.

## The Scoring Function

A natural question is where the scores can actually land. The end-game scoring function totals up every positive and negative contribution:

```
def FullPlayer.score (p : FullPlayer) : Int :=
  let animals := (p.dogs + p.sheep + p.donkeys +
                  p.wildBoars + p.cattle : Int)
  let grainPts := (((p.grain + p.fieldsWithGrain * 3) + 1) / 2 : Int)
  let vegPts := ((p.vegetables + p.fieldsWithVeg * 2) : Int)
  let rubyPts := (p.rubies : Int)
  let dwarfPts := ((p.dwarfs + p.offspring) : Int)
  let pasturePts := (p.smallPastures * 2 + p.largePastures * 4 : Int)
  let minePts := (p.oreMines * 3 + p.rubyMines * 4 : Int)
  let goldPts := (p.gold : Int)
  let furnPts := p.furnishings.foldl (fun acc fid =>
    acc + ((furnishingSpec fid).basePoints : Int)) (0 : Int)
  let missingTypes :=
    (if p.sheep == 0 then 1 else 0) + (if p.donkeys == 0 then 1 else 0) +
    (if p.wildBoars == 0 then 1 else 0) + (if p.cattle == 0 then 1 else 0)
  let unusedPenalty := (p.unusedMountain + p.unusedForest : Int)
  let beggingPenalty := (p.beggingMarkers * 3 : Int)
  let missingPenalty := (missingTypes * 2 : Int)
  animals + grainPts + vegPts + rubyPts + dwarfPts + pasturePts +
  minePts + goldPts + furnPts - unusedPenalty - beggingPenalty - missingPenalty
```

The theoretical floor is $-55$ points: a player who takes zero actions across all rounds gets +2 for their two starting dwarfs, $-8$ for missing all four farm animal types, $-22$ for 22 unused board spaces, and $-27$ from 9 begging markers at $-3$ points each. The theoretical ceiling is 202, computed by summing the independent maxima of every scoring category. But you can't reach 202, because every scoring category competes for the same 56 dwarf placements over 12 rounds. You cannot simultaneously furnish 12 caverns, build 5 mines, breed 20 animals, and sow 10 fields. The action budget is the binding constraint. The practical ceiling is around 140, achievable by furnishing rush when uncontested.

The theoretical range is 195 points (from $-55$ to 140), but the $-55$ floor requires literally doing nothing for 12 rounds, so it's not strategically meaningful. The practical floor across all archetypes is around 45 (peaceful farming in the worst setup), giving a practical range of about 95 points. The dominant strategy's own range is 80 points (60 to 140), proven as `dominant__strategy__variance`. The safety margin (`dominant__strategy__safety__margin`) is 115 points above the theoretical floor, but the more relevant number is the gap between furnishing rush's worst case (60) and the worst viable archetype's worst case (45): even in the most adversarial setup, choosing correctly gains you at least 15 points.

## Degenerate Combos

The formalization surfaced several combos that look broken in isolation. Four stand out.

The Beer Parlor converts grain to gold at a 3:2 ratio before scoring: every 2 grain becomes 3 gold. Without it, 20 grain scores 10 points (the grain formula is $\lceil n/2 \rceil$). With it, the same 20 grain becomes 30 gold. That's a 3x multiplier on the base grain value, and the gold stacks with every other scoring category. The `beer__parlor__max__gold` theorem proves `beerParlorGold 20 = 30`. The catch is accumulating 20 grain: you need aggressive sowing across multiple rounds, which means burning actions on Slash-and-Burn and Sustenance instead of excavating caverns.

Dogs have no cap on sheep-watching: $n$ dogs on one meadow guard $n+1$ sheep. Stack 10 dogs and a Weaving Parlor and you're looking at 51 points from a single tile combo. The catch is that acquiring 10 dogs requires roughly 6 expedition actions (dogs are strength-1 loot, so they're easy to get but each expedition burns an action), and those 6 actions aren't building caverns or growing your family.

The Writing Chamber prevents up to 7 points of losses for 2 stone. This sounds like a license to ignore entire scoring categories, and it is, except the categories you'd skip (unused spaces, missing animal types) are exactly the ones that furnishing rush covers naturally. The players who benefit most from Writing Chamber are the ones playing badly enough to accumulate large penalties, and at that point the cap at 7 means it barely dents a truly neglected board.

The Prayer Chamber is the most elegant. It gives 8 points for zero resource investment, but the bonus evaporates the moment any dwarf forges a weapon. The `prayer__chamber__vs__weapons` theorem proves this is a hard binary: 8 or 0, nothing in between. It creates a genuine strategic fork (peaceful play vs. expedition play) that the rest of the game's design carefully preserves.

None of these breaks the dominant strategy. Every combo faces the same binding constraint: 56 total actions with full family growth. You can produce flashy numbers in one scoring category, but you can't cover all of them. The vanilla furnishing rush, with its balanced approach to board coverage, family growth, and furnishing synergies, remains the ceiling. The combos are features, not bugs: they're what make the game worth replaying even after you know the dominant strategy, because they give you something to pivot into when the standard lines are blocked.

## Is the Game Solved?

A serious mathematician would call this "a formal analysis of a model, with the caveat that the model's parameters are estimated rather than computed." That is a legitimate and common pattern in applied math, operations research, and mechanism design. The proofs are machine-checked and airtight, but they are conditional on the payoff matrix. The theorems say: IF the payoff matrix is this, THEN furnishing rush is weakly dominant, the Nash equilibrium is unique, and the price of anarchy is 21/17. They do not say: the true expected score of an optimally-played furnishing rush against an optimally-played animal husbandry opponent is exactly 135. That number is an estimate derived from the formalized action economy, not from exhaustive search of the game tree.

How confident should you be in the estimate? The `min__dominance__margin__is__5` theorem proves the narrowest gap between furnishing rush and its closest competitor is 5 points. If the estimation methodology has a systematic bias of less than 5 points (plausible, given that the estimates are derived from the same scoring functions and action budgets used in actual play), the qualitative result holds. A correlated shift of 5+ points across an entire row of the matrix could break weak dominance in the mirror column, but would require the methodology to consistently overestimate furnishing rush's performance in self-contention scenarios. Given that the mirror penalty (85 vs. 130+ off-diagonal) already models severe contention, this seems unlikely.

There is a stronger structural argument that sidesteps the estimation question entirely. Each scoring channel has a maximum yield per action: furnishing converts actions to points at 6 points per action (excavation + furnishing tile with bonus multipliers), weapon/expedition at 4, and everything else at 3 or below. An `ActionAllocation` distributes the 37 productive actions (47 total placements minus \~10 food obligations) across the 6 channels. The `no__better__allocation__exists` theorem proves that for any valid allocation, the total score cannot exceed the furnishing-maximizing allocation's ceiling of 222 raw points (37 \* 6). Every action diverted from furnishing to the next-best channel (weapons) loses exactly 2 points; diversion to agriculture or mining loses 3. A plausible hybrid allocation of 20 furnishing + 17 weapons achieves at most 188 points, a 34-point deficit. No reallocation of the action budget can close this gap. The action economy structurally forbids a better strategy from existing.

```
theorem no_better_allocation_exists :
    forall a : ActionAllocation, a.valid ->
      a.rawScore <= furnishingMaxRawScore := by
  intro a h__valid
  simp only [ActionAllocation.rawScore, furnishingMaxRawScore,
             furnishingMaxAllocation, channelYieldPerAction,
             productiveActionsEstimate, totalPlacementsOneGrowthRound4]
  simp only [ActionAllocation.valid, ActionAllocation.totalActions,
             productiveActionsEstimate, totalPlacementsOneGrowthRound4] at h__valid
  omega
```

The proof is four lines. `omega` closes it because every channel yield is at most 6, so any allocation over 37 actions scores at most 37 \* 6 = 222. This result is independent of the payoff matrix. It depends only on the scoring rules and the action budget, both of which are formalized from the rulebook. Even if every payoff estimate were wrong by 50 points, this theorem would still hold: the furnishing channel has the highest marginal return per action, and the finite action budget makes that advantage insurmountable.

At the archetype level, yes, the game is solved. For any 2-player setup, regardless of which of the 2,880 configurations you draw, the model says the optimal pure strategy is furnishing rush. The proof does not depend on any specific card ordering or harvest marker placement. What I haven't done is compute the exact sequence of dwarf placements for each specific setup. That would require solving a game tree with roughly $10^{30}$ nodes. But the archetype-level result is the one that matters for actual play. You know the plan. The within-archetype decisions (which cavern to excavate first, which furnishing tile to prioritize) are tactical, not strategic. They don't change the answer.

The 3+ player variants remain open. The payoff matrix becomes a rank-3 tensor with 512 entries, action space contention shifts from binary to combinatorial, and the pure dominance result almost certainly fails, since furnishing tile scarcity under three-way competition makes mixed equilibria the likely outcome. If someone wants to formalize that, I would love to read the proof.

## Play the Dominant Strategy

**Play furnishing rush.** Excavate aggressively in rounds 1 through 4. Get Office Room early for the overhang bonuses. Get State Parlor for +4 per dwelling. Grow to 5 dwarfs and pick up Broom Chamber for +10 bonus. If your opponent does something different, you score 125 to 135 and they score 60 to 105. If your opponent also plays furnishing rush, you both land at 85, which is worse than the cooperative optimum of 210 but better than any unilateral deviation. You'll sit there at the table, both of you excavating caverns as fast as you can, both knowing that one of you could sacrifice 10 points to give the other 50, but neither willing to be the one who blinks.

Prisoner's dilemma in a cave. Uwe Rosenberg probably didn't intend this, but the formal analysis says it's there.

The code is built on Lean 4 v4.28.0 with Mathlib, modeling all 24 action spaces, all 48 furnishing tiles, the complete expedition loot table, the board grids, and the full 12-round 2-player harvest schedule. I think Caverna is a beautifully designed game, which is exactly why it was satisfying to find that even under formal analysis the strategic structure holds together so well. A badly designed game would have a trivially dominant strategy that makes the game boring. Caverna's dominant strategy comes with a 19% welfare tax on the mirror matchup, which means the game is always more interesting when your opponent does something unexpected (the Nash equilibrium is rarely where the fun is, in games or in life), which means in practice people don't always play the dominant strategy, which means the game stays fun.

Good game design, it turns out, is robust to being solved. Now you know the optimal play. Whether you follow it is, thankfully, not a theorem.
