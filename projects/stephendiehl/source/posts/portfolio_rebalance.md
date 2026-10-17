---
date: "2024-02-05"
tags: ["markets", "python", "cvxopt"]
url: "https://www.stephendiehl.com/posts/portfolio_rebalance/"
---

# Portfolio Rebalancing

Continuing on our topic of portfolio theory, we've already discussed how to optimize a portfolio given a set of constraints. Now we'll discuss how to use cvxopt to rebalance a portfolio to maintain the desired asset allocation.

For example we're going to start with a simple portfolio that exists in a current portfolio and we'll rebalance it to a target portfolio. While the optimization techniques we've discussed in the last post determine optimal portfolio weights, maintaining these weights over time requires careful consideration of transaction costs and market impact. We will need to change the portfolio weights over time to either reflect a new thesis or in response to changing market conditions.

The rebalancing problem can be formulated as a quadratic programming problem that balances three competing objectives:

1.  Minimizing the distance from target weights
2.  Minimizing transaction costs
3.  Maintaining risk-return characteristics

The mathematical formulation looks like this:

$$\begin{array}{ll}
\text{minimize} & \lambda_1(w - w_t)^T\Sigma(w - w_t) + \lambda_2|w - w_c|_2^2 + \lambda_3|w - w_c|_1 \\
\text{subject to} & \mathbf{1}^T w = 1 \\
& w_i \geq 0, \quad i = 1,\ldots,n
\end{array}$$

Where:

- $w_t$ are the target weights from our optimization
- $w_c$ are the current weights
- $\lambda_1$ controls the risk penalty
- $\lambda_2$ controls the L2 transaction cost (quadratic cost)
- $\lambda_3$ controls the L1 transaction cost (linear cost)

Here's a practical implementation:

``` python
import numpy as np
from cvxopt import matrix, solvers

def rebalance_portfolio(current_weights, target_weights, covariance_matrix, lambda_risk=1.0, lambda_trade=1.0, lambda_cost=0.01):
    n = len(current_weights)

    # Convert inputs to cvxopt format
    P = matrix(lambda_risk * covariance_matrix.values +
              lambda_trade * np.eye(n))

    # Linear cost term
    q = matrix(lambda_cost * np.ones(n) -
              2 * lambda_trade * current_weights)

    # Constraints
    G = matrix(np.vstack((-np.eye(n),  # Long only constraint
                         np.eye(n))))   # Upper bound constraint
    h = matrix(np.hstack((np.zeros(n),  # w_i >= 0
                         np.ones(n))))   # w_i <= 1

    A = matrix(1.0, (1, n))  # Sum of weights = 1
    b = matrix(1.0)

    # Solve the QP problem
    sol = solvers.qp(P, q, G, h, A, b)

    if sol['status'] != 'optimal':
        raise ValueError("Optimization failed to converge")

    new_weights = np.array(sol['x']).flatten()

    # Calculate turnover
    turnover = np.sum(np.abs(new_weights - current_weights))

    return new_weights, turnover
```

And as a simple example, let's rebalance a portfolio with 4 stocks that we want to rebalance to an equal 25% weight in each stock:

``` python
current_weights = np.array([0.3, 0.3, 0.2, 0.2])  # Current portfolio weights
target_weights = np.array([0.25, 0.25, 0.25, 0.25])  # Target weights

new_weights, turnover = rebalance_portfolio(
    current_weights,
    target_weights,
    covariance_matrix
)

print("Rebalancing Results:")
print("Current weights:", current_weights)
print("Target weights:", target_weights)
print("New weights:", new_weights.round(3))
print("Turnover:", f"{turnover:.2%}")
```

This will solve for the amount of each stock to buy or sell to get to the target weights. First, it accounts for transaction costs, incorporating both linear costs, which are proportional to trade size, and quadratic costs that reflect market impact. The rebalanced portfolio is designed to maintain risk characteristics that are similar to those of the target portfolio. Finally, turnover control is a key aspect of the optimization process, as it seeks to balance the benefits of rebalancing with the associated costs.

We can also tune the constraints with the lambda parameters:

- Higher `lambda_risk` puts more emphasis on matching the risk characteristics
- Higher `lambda_trade` reduces turnover
- Higher `lambda_cost` makes the optimization more sensitive to transaction costs

**Sector Exposure**

In reality we would want to have a lot more additional constraints to ensure that we are meeting our risk and return objectives. In practice, you might want to add additional constraints such as:

- Sector exposure limits
- Maximum position sizes
- Minimum trade sizes
- Trading costs that vary by asset

These can be incorporated by adding even more constraints to the optimization problem. As an example, let's add the following constraints:

- No sector can exceed 30% of the portfolio
- Total turnover (sum of absolute changes) cannot exceed 20%
- Individual position sizes remain between 0% and 100%
- The sum of weights equals 100%

Let's work with a more realistic example using 10 stocks from different sectors of the S&P 500:

``` python
import pandas as pd

stocks = {
    'AAPL': 'Technology',
    'MSFT': 'Technology',
    'JPM':  'Financials',
    'T':    'Financials',
    'JNJ':  'Healthcare',
    'ABBV': 'Healthcare',
    'PG':   'Consumer Staples',
    'KO':   'Consumer Staples',
    'XOM':  'Energy',
    'CVX':  'Energy',
    'HD':   'Consumer Discretionary',
    'NEE':  'Utilities',
    'LIN':  'Materials',
    'UNP':  'Industrials',
    'VZ':   'Communications'
}

# Read in historical stock prices from a CSV file
data = pd.read_csv('stock_prices.csv', index_col='Date', parse_dates=True)

# Calculate returns and covariance matrix
returns = data.pct_change().dropna()
covariance_matrix = returns.cov() * 252  # Annualized covariance
```

Now let's enhance our rebalancing function to include sector constraints and more realistic transaction costs:

``` python
def rebalance_portfolio_with_constraints(
    current_weights,
    target_weights,
    covariance_matrix,
    sector_map,
    transaction_costs,
    max_sector_exposure=0.30,
    max_turnover=0.20,
    lambda_risk=1.0,
    lambda_trade=1.0
):
    n = len(current_weights)

    # Create sector constraint matrix
    unique_sectors = list(set(sector_map.values()))
    sector_constraints = np.zeros((len(unique_sectors), n))

    for i, sector in enumerate(unique_sectors):
        for j, stock in enumerate(sector_map.keys()):
            if sector_map[stock] == sector:
                sector_constraints[i, j] = 1

    # Convert inputs to cvxopt format
    P = matrix(lambda_risk * covariance_matrix.values +
              lambda_trade * np.eye(n))

    # Linear cost term including transaction costs
    q = matrix(-2 * lambda_trade * current_weights)

    # Constraints matrix
    G = matrix(np.vstack([
        -np.eye(n),                # Long only constraint
        np.eye(n),                 # Upper bound constraint
        sector_constraints,        # Sector exposure constraints
        -sector_constraints,       # Minimum sector exposure
        np.eye(n),                 # Positive turnover
        -np.eye(n)                 # Negative turnover
    ]))

    # Constraints vector
    h = matrix(np.hstack([
        np.zeros(n),              # Long only
        np.ones(n),               # Upper bound
        np.repeat(max_sector_exposure, len(unique_sectors)),  # Max sector
        np.zeros(len(unique_sectors)),                        # Min sector
        current_weights + max_turnover,  # Max positive turnover
        -current_weights + max_turnover  # Max negative turnover
    ]))

    # Sum of weights = 1
    A = matrix(1.0, (1, n))
    b = matrix(1.0)

    # Solve the QP problem
    sol = solvers.qp(P, q, G, h, A, b)

    if sol['status'] != 'optimal':
        raise ValueError("Optimization failed to converge")

    new_weights = np.array(sol['x']).flatten()
    turnover = np.sum(np.abs(new_weights - current_weights))

    # Calculate transaction costs
    total_cost = sum(abs(new_weights[i] - current_weights[i]) * transaction_costs[ticker]
                    for i, ticker in enumerate(sector_map.keys()))

    return new_weights, turnover, total_cost
```

Ok let's run this with some real data. We'll use the current portfolio weights and the target portfolio weights from the last post:

``` python
# What we have
current_portfolio = {
    'AAPL': 0.15, # 15% Apple
    'MSFT': 0.12, # 12% Microsoft
    'JPM':  0.10, # 10% JPMorgan
    'T':    0.08, # 8%  AT&T
    'JNJ':  0.12, # 12% Johnson & Johnson
    'ABBV': 0.08, # 8%  AbbVie
    'PG':   0.10, # 10% Procter & Gamble
    'KO':   0.08, # 8%  Coca-Cola
    'XOM':  0.09, # 9%  Exxon Mobil
    'CVX':  0.08, # 8%  Chevron
    'HD':   0.07, # 7%  Home Depot
    'NEE':  0.06, # 6%  NextEra Energy
    'LIN':  0.05, # 5%  Linde
    'UNP':  0.06, # 6%  Union Pacific
    'VZ':   0.05  # 5%  Verizon
}

# What we want
target_portfolio = {
    'AAPL': 0.10, # 10% Apple
    'MSFT': 0.10, # 10% Microsoft
    'JPM':  0.10, # 10% JPMorgan
    'T':    0.10, # 10% AT&T
    'JNJ':  0.10, # 10% Johnson & Johnson
    'ABBV': 0.10, # 10% AbbVie
    'PG':   0.10, # 10% Procter & Gamble
    'KO':   0.10, # 10% Coca-Cola
    'XOM':  0.10, # 10% Exxon Mobil
    'CVX':  0.10, # 10% Chevron
    'HD':   0.0,  # 0%  Home Depot
    'NEE':  0.0,  # 0%  NextEra Energy
    'LIN':  0.0,  # 0%  Linde
    'UNP':  0.0,  # 0%  Union Pacific
    'VZ':   0.0   # 0%  Verizon
}

# Dummy transaction costs (will vary by stock liquidity)
transaction_costs = {
    'AAPL': 0.001,   # 10 bps
    'MSFT': 0.001,   # 10 bps
    'JPM':  0.0012,  # 12 bps
    'T':    0.0012,  # 12 bps
    'JNJ':  0.001,   # 10 bps
    'ABBV': 0.001,   # 10 bps
    'PG':   0.001,   # 10 bps
    'KO':   0.001,   # 10 bps
    'XOM':  0.0015,  # 15 bps
    'CVX':  0.0015,  # 15 bps
    'HD':   0.0012,  # 12 bps
    'NEE':  0.0015,  # 15 bps
    'LIN':  0.0018,  # 18 bps
    'UNP':  0.0015,  # 15 bps
    'VZ':   0.0018   # 18 bps
}

# Example usage
current_weights = np.array([current_portfolio[ticker] for ticker in stocks.keys()])
target_weights = np.array([target_portfolio[ticker] for ticker in stocks.keys()])


new_weights, turnover, total_cost = rebalance_portfolio_with_constraints(
    current_weights=current_weights,
    target_weights=target_weights,
    covariance_matrix=covariance_matrix,
    sector_map=stocks,
    transaction_costs=transaction_costs,
    max_sector_exposure=0.30,
    max_turnover=0.20
)

# Print results with sector analysis
results = pd.DataFrame({
    'Stock': list(stocks.keys()),
    'Sector': list(stocks.values()),
    'Current Weight': current_weights,
    'Target Weight': target_weights,
    'New Weight': new_weights.round(4)
})

print("Rebalancing Results:")
print(results)
print(f"Turnover: {turnover:.2%}")
print(f"Transaction Costs: ${total_cost*1000000:.2f} per $1M traded")

# Sector exposure analysis
print("Sector Exposures:")
sector_exposures = results.groupby('Sector')['New Weight'].sum().round(4)
print(sector_exposures)
```

Ok this is a lot more realistic. We can see that the solution is respecting the sector constraints and the transaction costs are being minimized. Quite often the expected return of the portfolio is lower than the target return because of the additional constraints, especially with large position sizes.

A more complicated model would also take into account the slippage that occurs when making large trades and the potential impact on the market price of the stocks. This isn't directly a constraint of the optimization problem, but it is something we can use other models to help mitigate.
