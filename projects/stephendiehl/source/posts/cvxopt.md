---
date: "2024-02-01"
tags: ["markets", "portfolio theory", "cvxopt", "python", "mathematics"]
url: "https://www.stephendiehl.com/posts/cvxopt/"
---

# Portfolio Theory with CVXOPT

In this tutorial, we will use the `cvxopt` library in Python to implement Markowitz Portfolio Theory, which allows us to optimize a portfolio of assets by calculating the efficient frontier. The efficient frontier represents the set of optimal portfolios that offer the highest expected return for a given level of risk (standard deviation).

In plain English, what we want is a mix of investments that generally go up over time, but don't all tank at once when things go bad. Ideally, when some investments are going down, we want others that tend to go up to balance things out. It's like not putting all your eggs in one basket, but being really smart about which baskets you choose and how many eggs go in each one. We'll go through three different ways of being "smarter" about our portfolio choice.

1.  **Classical Portfolio Optimization** - This is the simplest approach, where we just try to find the best risk-return trade-off by considering the contributions of each asset to the overall portfolio.
2.  **Factor Models** - This approach is more sophisticated and takes into account the fact that assets often move together in response to common economic drivers.
3.  **Black-Litterman Model** - This approach is a Bayesian take on portfolio optimization that combines market equilibrium returns with a "thesis" or view on which assets will perform well to produce a customized portfolio that balances risk and return according to the investor's risk aversion and views.

Some short background, what we now colloquially call *modern portfolio theory* was developed by Harry Markowitz in 1952 and forms the theoretical basis in which we now discuss optimal portfolios and diversification. The central insight of MPT is that an asset's risk and return shouldn't be evaluated in isolation, but rather by how it contributes to a portfolio's overall risk and return. The big idea is that we can construct portfolios that have the best risk-return trade-off by considering the contributions of each asset to the overall portfolio, but the question is how much, and that's where we can use modern tools like convex optimization solvers to help us out.

You'll need to install the `cvxopt` library, which can be done via pip:

``` bash
pip install numpy pandas matplotlib cvxopt PyPortfolioOpt
```

I'm not going to explain everything, as this is a tutorial about code and convex optimization, not finance. But if you want to learn more about the finance, I recommend the following resources:

- *Modern Portfolio Theory* - Grinold and Kahn
- *Paul Wilmott on Quantitative Finance* - Paul Wilmott

We're also going to use the public data from Aswath Damodaran's website, which you can download [here](http://www.stern.nyu.edu/~adamodar/pc/datasets/histretSP.xls). In real life you'd get this data from Bloomberg or Reuters.

**Some Theory**

At its core, portfolio theory deals with the relationship between risk and return. Return is typically measured as the expected value (mean) of historical returns, representing our best estimate of what we might earn in the future. Risk, on the other hand, is quantified through the standard deviation of returns, also known as *volatility*. This volatility measure captures the uncertainty in our expected returns. We call *weights* the fraction of our portfolio allocated to each asset (for example, 60% of our portfolio in stocks and 40% in bonds).

Volatility plays a crucial role in investment decisions for several reasons. First, it represents the market's perception of risk – assets with higher volatility are generally considered riskier and thus should offer higher expected returns. Second, volatility directly impacts trading costs and opportunities, as more volatile assets typically have wider bid-ask spreads and require larger position sizes to achieve the same economic exposure. Finally, volatility affects an investor's ability to meet future cash flow needs, as high volatility can force liquidation at inopportune times.

The power of MPT comes from understanding how assets move together, measured through their correlation. The portfolio risk is determined not just by the individual asset risks, but by how they move together. This relationship is captured in the portfolio risk formula:

$$\sigma = \sqrt{w_1^2 \sigma_1^2 + w_2^2 \sigma_2^2 + 2w_1w_2\sigma_1\sigma_2\rho_{12}}$$

In this equation, $w$ represents weights, $\sigma$ represents individual asset volatilities, and $\rho$ represents the correlation coefficient between the assets. This formula reveals something remarkable: by combining assets with less than perfect positive correlation, we can achieve better risk-adjusted returns than by holding either asset alone. For a portfolio of n assets, the expected return is a weighted sum of individual asset returns:

$$E(R_p) = \sum_{i=1}^n w_i E(R_i)$$

where $w_i$ represents the weight of asset i and $E(R_i)$ is its expected return. This is the simpler part of the theory. The portfolio variance (risk squared) is given by:

$$\sigma_p^2 = \sum_{i=1}^n \sum_{j=1}^n w_i w_j \sigma_{ij}$$

where $\sigma*{ij}$ represents the covariance between assets i and j. When $i=j$, this becomes the variance of asset i. This equation captures the critical insight that portfolio risk depends not just on individual asset risks but on how assets move together. This can be captured in the so-called \_covariance matrix\*, which shows how different investments tend to move together. Think of it as a big table - if you look at any two investments in the table, their intersection tells you whether they tend to move in the same direction (positive relationship), opposite directions (negative relationship), or independently of each other (no relationship). The diagonal of this table shows how much each investment bounces around on its own (its volatility). This is a useful view because it helps us understand not just how risky individual investments are, but how they might work together to reduce overall portfolio risk.

$$\Sigma = \begin{bmatrix}
\sigma_{11} & \sigma_{12} & \cdots & \sigma_{1n} \\
\sigma_{21} & \sigma_{22} & \cdots & \sigma_{2n} \\
\vdots & \vdots & \ddots & \vdots \\
\sigma_{n1} & \sigma_{n2} & \cdots & \sigma_{nn}
\end{bmatrix}$$

The matrix is symmetric and the diagonal elements are the variances of the individual assets. The off-diagonal elements are the covariances between the assets. In our case, $\Sigma$ is a $4$x$4$ matrix because we have four investments: U.S. Treasury Bonds (denoted $A$), Corporate Bonds (denoted $B$), Stocks (denoted $C$), and T-Bills (denoted $D$).

$$\Sigma = \begin{bmatrix}
\sigma_{A,A} & \sigma_{A,B} & \sigma_{A,C} & \sigma_{A,D} \\
\sigma_{B,A} & \sigma_{B,B} & \sigma_{B,C} & \sigma_{B,D} \\
\sigma_{C,A} & \sigma_{C,B} & \sigma_{C,C} & \sigma_{C,D} \\
\sigma_{D,A} & \sigma_{D,B} & \sigma_{D,C} & \sigma_{D,D}
\end{bmatrix}$$

Now in order to find the optimal portfolio, we need to solve the problem of minimizing the portfolio risk $\sigma_p^2$ subject to constraints: $\sum_{i=1}^n w_i = 1$ (weights sum to 1), $E(R_p) = R_{target}$ (target return constraint), and often $w_i \geq 0$ (no short selling).

**Classical Portfolio Optimization**

Using the theory above, our model aims to find the optimal portfolio allocation by balancing two competing objectives:

1.  Maximizing expected returns
2.  Minimizing risk (variance)

The optimization problem is formulated as:

$$\begin{array}{ll}
\text{maximize} & \mu^T w - \gamma w^T\Sigma w \\
\text{subject to} & \mathbf{1}^T w = 1, \quad w \in \mathcal{W}
\end{array}$$

Where:

- $w \in \mathbb{R}^n$ is the portfolio allocation vector (the weights for each asset)
- $\mu$ is the vector of expected returns
- $\Sigma$ is the covariance matrix
- $\gamma > 0$ is the risk aversion parameter
- $\mathcal{W}$ is the set of allowed portfolios (e.g., $\mathcal{W} = \mathbb{R}_+^n$ for long-only portfolios)
- $\mathbf{1}^T w = 1$ ensures the weights sum to 1

The objective function $\mu^T w - \gamma w^T\Sigma w$ represents the risk-adjusted return where:

- $\mu^T w$ is the expected portfolio return
- $w^T\Sigma w$ is the portfolio variance (risk)
- $\gamma$ controls the trade-off between return and risk

By varying the risk aversion parameter $\gamma$, you can trace out the efficient frontier, which shows the optimal risk-return trade-offs available to investors. A higher $\gamma$ results in more conservative portfolios (lower risk, lower return), while a lower $\gamma$ leads to more aggressive portfolios (higher risk, higher return). The same risk-return trade-off can also be obtained by fixing a target return and minimizing risk, or fixing a risk level and maximizing return.

The so-called *efficient frontier* represents the set of portfolios that offer the highest expected return for a given level of risk, or conversely, the lowest risk for a given expected return. This concept is powerful because it shows that there exists an optimal set of portfolios that dominate all others – any portfolio not on the efficient frontier can be improved by either increasing return while maintaining the same risk or reducing risk while maintaining the same return.

In short, it's the set of portfolios that given a target risk and return any rational investor would choose. And the efficient frontier is the boundary of the set of achievable portfolios. And we can can empirically estimate the efficient frontier by calculating the returns and risks of portfolios with different weights.

Theory aside, when implementing portfolio optimization in practice, several real-world considerations come into play. The theory assumes we know the true expected returns and covariances of assets, but in reality, we must estimate these from historical data. These estimates contain uncertainty, which can lead to optimization error.

Also, the standard naive theory assumes frictionless markets with no transaction costs and perfect divisibility of assets. Real markets have friction in the form of bid-ask spreads, transaction costs, and minimum trade sizes. These practical limitations often necessitate additional constraints in the optimization problem.

**Implementation**

``` python
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from cvxopt import matrix, solvers

# Download historical returns data
data_sheet = "Returns by year"
skiprows = 17  # Skip header rows
skipfooter = 1  # Skip footer rows

# Download the data from NYU Stern's website
df = pd.read_excel(
    'http://www.stern.nyu.edu/~adamodar/pc/datasets/histretSP.xls',
    sheet_name=data_sheet,
    skiprows=skiprows,
    skipfooter=skipfooter
)

# Select and rename the relevant columns
columns_of_interest = {
    'S&P 500 (includes dividends)': 'SP500',
    '3-month T.Bill': 'T_Bill',
    'US T. Bond': 'T_Bond',
    'Baa Corporate Bond': 'Corp_Bond'
}

# Clean and prepare the data
df = df[columns_of_interest.keys()].rename(columns=columns_of_interest)

# Convert percentages to decimals
df = df / 100

# Calculate basic statistics
returns = df.mean()
risks = df.std()
correlation_matrix = df.corr()
covariance_matrix = df.cov()

print("Annual Returns:")
print(returns)
print("Annual Risks (Standard Deviation):")
print(risks)
print("Correlation Matrix:")
print(correlation_matrix)
```

After calculating the basic statistics, let's add the optimization code which will calculate the minimum volatility portfolio and the portfolio weights.

``` python
solvers.options['show_progress'] = False  # Suppress solver output

# Convert our data to the format CVXOPT expects
n = len(returns)  # number of assets
P = matrix(covariance_matrix.values)  # covariance matrix
q = matrix(0.0, (n, 1))
G = matrix(-np.eye(n))  # negative identity matrix for >= 0 constraint
h = matrix(0.0, (n, 1))
A = matrix(1.0, (1, n))  # constraint that weights sum to 1
b = matrix(1.0)

# Solve for minimum volatility portfolio
sol = solvers.qp(P, q, G, h, A, b)
weights = np.array(sol['x']).flatten()

# Calculate portfolio metrics
min_vol_ret = np.sum(returns * weights)
min_vol_risk = np.sqrt(np.dot(weights.T, np.dot(covariance_matrix, weights)))

# Calculate efficient frontier
target_returns = np.linspace(min_vol_ret, max(returns), 100)
frontier_risks = []
frontier_weights = []

for target_ret in target_returns:
    P = matrix(covariance_matrix.values)
    q = matrix(0.0, (n, 1))
    G = matrix(np.vstack((-np.eye(n), -returns)))
    h = matrix(np.hstack((np.zeros(n), -target_ret)))
    A = matrix(1.0, (1, n))
    b = matrix(1.0)

    sol = solvers.qp(P, q, G, h, A, b)
    if sol['status'] == 'optimal':
        weights = np.array(sol['x']).flatten()
        risk = np.sqrt(np.dot(weights.T, np.dot(covariance_matrix, weights)))
        frontier_risks.append(risk)
        frontier_weights.append(weights)

# Plot the efficient frontier
plt.figure(figsize=(10, 6))
plt.plot(frontier_risks, target_returns, 'b-', label='Efficient Frontier')

# Plot individual assets
for i, asset in enumerate(returns.index):
    plt.scatter(risks[i], returns[i], marker='o', label=asset)

# Plot minimum volatility portfolio
plt.scatter(min_vol_risk, min_vol_ret, color='red', marker='*',
           s=200, label='Minimum Volatility')

plt.xlabel('Risk (Standard Deviation)')
plt.ylabel('Expected Return')
plt.title('Efficient Frontier')
plt.legend()
plt.grid(True)
plt.show()

# Print minimum volatility portfolio allocation
print("Minimum Volatility Portfolio Allocation:")
for asset, weight in zip(returns.index, weights):
    print(f"{asset}: {weight:.2%}")

print(f"Portfolio Return: {min_vol_ret:.2%}")
print(f"Portfolio Risk: {min_vol_risk:.2%}")
```

**Factor Models**

Factor models represent an extension to portfolio theory that provide a more structured way to understand and decompose asset returns. While MPT focuses on the relationship between overall portfolio risk and return, factor models break down asset returns into systematic components (driven by common factors) and idiosyncratic components (specific to each asset). This decomposition helps us better understand the sources of risk and return in their portfolios. For example, instead of just knowing that a stock has a certain expected return and volatility, factor models might reveal that 40% of its return variation comes from market exposure, 20% from interest rate sensitivity, and the remaining 40% from company-specific events.

Interest rate exposure is a systematic risk factor since it affects all assets. Market exposure is also a systematic risk factor since it affects all assets, broadly as the S&P 500 trends upwards or downwards it can affect all assets. And then company-specific events are idiosyncratic risk since they are specific to each asset (i.e. new product launches, earnings reports, accounting scandals, etc.).

The theoretical foundation of factor models rests on the observation that assets often move together in response to common economic drivers. These common movements can be captured by a set of factors – such as market returns, interest rates, inflation, or more abstract statistical factors. The famous Capital Asset Pricing Model (CAPM) can be viewed as the simplest factor model, using only the market return as a single factor. More sophisticated models like the Fama-French three-factor model add size and value factors, while modern approaches might include dozens of factors capturing various market anomalies and risk premia. The key insight is that by identifying these common factors, we can build more robust portfolios that are deliberately exposed to or protected from specific economic risks.

This represents the mathematical optimization problem:

$$\begin{array}{ll}
\text{maximize} & \mu^T w - \gamma(f^T \tilde{\Sigma} f + w^T D w) \\
\text{subject to} & \mathbf{1}^T w = 1 \\
& |w|_1 \leq L^{\text{max}} \\
& f = F^T w
\end{array}$$

Where:

- $n$ is the number of assets
- $w \in \mathbb{R}^n$ is the portfolio allocation vector
- $f \in \mathbb{R}^k$ represents the factor exposures
- $\mu$ is the vector of expected returns
- $F$ is the factor loading matrix
- $\tilde{\Sigma}$ is the factor covariance matrix
- $D$ is the diagonal matrix of idiosyncratic risks
- $\gamma = 0.1$ is the risk aversion parameter
- $L^{\text{max}} = 2$ is the leverage limit

In cvxopt, the optimization problem is formulated as:

``` python
# Variables:
w = cp.Variable(n)  # Portfolio weights
f = F.T*w           # Factor exposures

# Objective function:
ret = mu.T*w       # Expected return
risk = cp.quad_form(f, Sigma_tilde) + cp.quad_form(w, D)  # Factor model risk
objective = cp.Maximize(ret - gamma*risk)

# Constraints:
constraints = [
    cp.sum(w) == 1,        # Weights sum to 1
    cp.norm(w, 1) <= Lmax  # Leverage limit
]
```

Factor models complement MPT by providing a more nuanced framework for portfolio optimization. While MPT uses historical correlations between assets to optimize portfolios, factor models can potentially provide more stable and forward-looking estimates of how assets will move together. This is particularly valuable when historical data is limited or when market conditions are changing. Furthermore, factor models allow investors to construct portfolios with specific factor tilts – for instance, maximizing exposure to value while minimizing market beta – which goes beyond the simple risk-return optimization of traditional MPT. This makes factor models particularly valuable for institutional investors who need to manage specific risk exposures or implement sophisticated investment strategies while maintaining the core insights of portfolio diversification from MPT.

``` python
# Prepare factor data
# Using S&P 500 as market factor and T-Bill rate as interest rate factor
factors = df[['SP500', 'T_Bill']]
assets = df[['T_Bond', 'Corp_Bond']]  # Assets to model

# Run factor regression for each asset
factor_betas = []
idiosyncratic_vars = []

for asset in assets.columns:
    # Perform linear regression
    X = factors
    y = assets[asset]
    beta = np.linalg.inv(X.T @ X) @ X.T @ y

    # Calculate residuals and their variance
    residuals = y - X @ beta
    idio_var = np.var(residuals)

    factor_betas.append(beta)
    idiosyncratic_vars.append(idio_var)

# Convert to numpy arrays
factor_betas = np.array(factor_betas)
idiosyncratic_vars = np.diag(idiosyncratic_vars)

# Calculate factor covariance matrix
factor_cov = factors.cov()

# Construct the factor-based covariance matrix
factor_based_cov = (factor_betas @ factor_cov @ factor_betas.T) + idiosyncratic_vars

# Optimize portfolio using factor-based covariance
n = len(assets.columns)
P = matrix(factor_based_cov)
q = matrix(0.0, (n, 1))
G = matrix(-np.eye(n))
h = matrix(0.0, (n, 1))
A = matrix(1.0, (1, n))
b = matrix(1.0)

# Solve for minimum volatility portfolio using factor-based covariance
sol = solvers.qp(P, q, G, h, A, b)
factor_weights = np.array(sol['x']).flatten()

# Calculate portfolio metrics using factor model
factor_portfolio_risk = np.sqrt(np.dot(factor_weights.T,
                                     np.dot(factor_based_cov, factor_weights)))

# Print results
print("Factor Model Analysis:")
print("Factor Betas:")
for i, asset in enumerate(assets.columns):
    print(f"{asset}:")
    print(f"Market Beta: {factor_betas[i][0]:.3f}")
    print(f"Interest Rate Beta: {factor_betas[i][1]:.3f}")

print("Optimal Portfolio Weights (Factor Model):")
for asset, weight in zip(assets.columns, factor_weights):
    print(f"{asset}: {weight:.2%}")
print(f"Portfolio Risk (Factor Model): {factor_portfolio_risk:.2%}")

# Compare factor-based and sample covariance matrices
print("Covariance Matrix Comparison:")
print("Factor-Based Covariance:")
print(pd.DataFrame(factor_based_cov,
                  index=assets.columns,
                  columns=assets.columns))

print("Sample Covariance:")
print(assets.cov())
```

**Black-Litterman Model**

The culmination of the analysis comes with the Black-Litterman model implementation, which represents a more practical approach used by institutional investors. This is a Bayesian model that starts with market equilibrium returns derived from market capitalizations, rather than historical returns, and allows investors to incorporate their specific views with varying levels of confidence. The model demonstrates how to combine market "wisdom" (through equilibrium returns) with investor insights (through explicit views) to create more intuitive and stable portfolio allocations.

The core of Black-Litterman is captured in the posterior return equation:

$$E(R) = [(τΣ)^{-1} + P^T Ω^{-1} P]^{-1}[(τΣ)^{-1} Π + P^T Ω^{-1} Q]$$

Where:

- $R$ is a $N$x$1$ vector of returns
- $Q$ is a $K$x$1$ vector of views.
- $\Sigma$ is the $N$x$N$ covariance matrix of asset returns
- $E(R)$ is a $N$x$1$ vector of expected returns, where $N$ is the number of assets.
- $P$ is the $K$x$N$ picking matrix which maps views to the universe of assets.
- $\Omega$ is the $K$x$K$ uncertainty matrix of views.
- $\Pi$ is the $N$x$1$ vector of prior expected returns.
- $\tau$ is a scalar tuning constant.

This equation combines market equilibrium returns ($\Pi$) with investor views ($Q$) using Bayesian statistics. The model also produces a posterior covariance matrix

$$\hat{\Sigma} = \sigma^2 + [(\tau \sigma^2)^{-1} + P^T \Omega^{-1} P]^{-1}$$

that accounts for uncertainty in both the market prior and investor views. The market-implied returns are calculated as

$$(\Pi = \delta \sigma w_{mkt}),$$

where the market risk aversion parameter $\delta$ is derived from

$$\delta = (R-R_f)/\sigma^2$$

Think of the BL model as a sophisticated way of saying "I have some ideas about where the market is going, but I'm not entirely sure." The model starts with what the market is implying through current prices - this is like getting the wisdom of the crowd. Then, you add your own views, like "I think tech stocks will outperform" or "bonds will struggle this year." The clever part is that you can say how confident you are in each view and you can incorporate your uncertainty in the views.

The model then does something similar to what we do naturally: if you're very confident in your view and it differs from the market consensus, the final expectation will move significantly toward your view. If you're less confident, it will stay closer to what the market implies. Even more impressively, if you have a view on one asset (like tech stocks), the model will automatically adjust related assets (like semiconductor manufacturers) in a sensible way based on their historical relationships.

The beauty of this approach is that it produces much more reasonable portfolio allocations than traditional methods. Instead of putting all your money in the asset with the highest historical return (which traditional optimization often suggests), it creates a balanced portfolio that reflects both market wisdom and your personal insights, while accounting for how confident you are in each.

Unlike the code above, this model addresses many of the limitations of traditional mean-variance optimization, such as extreme allocations and high sensitivity to input parameters, making it particularly valuable for real-world portfolio management. We'll use the implementation in PyPortfolioOpt library.

Our code is going to use the so-called *Idzorek method* for estimating the views uncertainty matrix $\Omega$. I'm not going to dive into the details of the method here, but it's a sophisticated way to estimate the uncertainty in the views that the underlying library will take care of Our example thesis for this code is:

- **Market-Implied Returns:** Instead of using historical returns, we start with market equilibrium returns derived from market capitalizations. This assumes that current market prices reflect collective wisdom.
- **Investor Views:** We incorporate specific views on expected returns:
  - Bullish view on S&P 500 (12% return)
  - Conservative view on Treasury bonds (4% return)
  - Moderate view on Corporate bonds (6% return)
- **View Confidence:** We specify different confidence levels for each view using Idzorek's method:
  - 60% confidence in S&P 500 view
  - 70% confidence in Treasury bond view
  - 50% confidence in Corporate bond view
- **Posterior Estimates:** We combine the prior (market-implied) returns with our views to produce posterior estimates of returns and covariance.

``` python
from pypfopt.black_litterman import BlackLittermanModel
from pypfopt.efficient_frontier import EfficientFrontier
from pypfopt import black_litterman

# Calculate market-implied risk aversion
delta = black_litterman.market_implied_risk_aversion(df['SP500'])

# Market capitalization weights (example values - you should use real market caps)
market_caps = {
    'SP500': 1e6,      # S&P 500
    'T_Bill': 5e5,     # T-Bills
    'T_Bond': 7e5,     # Treasury Bonds
    'Corp_Bond': 3e5   # Corporate Bonds
}

# Calculate prior (market-implied) returns
prior = black_litterman.market_implied_prior_returns(
    market_caps,
    delta,
    covariance_matrix
)

# Specify your views
viewdict = {
    'SP500': 0.12,        # Bullish on S&P 500: 12% return
    'T_Bond': 0.04,       # Treasury bonds will return 4%
    'Corp_Bond': 0.06     # Corporate bonds will return 6%
}

# Confidence in each view (from 0 to 1)
view_confidences = [0.6, 0.7, 0.5]

# Create the Black-Litterman model
bl = BlackLittermanModel(
    cov_matrix=covariance_matrix,
    pi=prior,
    absolute_views=viewdict,
    omega="idzorek",
    view_confidences=view_confidences
)

# Calculate posterior returns and covariance
ret_bl = bl.bl_returns()
cov_bl = bl.bl_cov()


# Optimize the portfolio using the Black-Litterman estimates

ef_bl = EfficientFrontier(ret_bl, cov_bl)
weights_bl = ef_bl.max_sharpe()
cleaned_weights = ef_bl.clean_weights()

# Print the results
print("Black-Litterman Portfolio Allocation:")
for asset, weight in cleaned_weights.items():
    print(f"{asset}: {weight:.2%}")

# Calculate and print portfolio performance metrics
expected_return, volatility, sharpe = ef_bl.portfolio_performance()
print(f"Expected annual return: {expected_return:.2%}")
print(f"Annual volatility: {volatility:.2%}")
print(f"Sharpe Ratio: {sharpe:.2f}")

# Compare posterior vs prior returns
print("Return Estimates Comparison:")
print("Prior (Market-Implied) Returns:")
print(prior)
print("Posterior (Black-Litterman) Returns:")
print(ret_bl)
```

When we run the efficient frontier optimizer with the Black-Litterman posterior returns and covariance matrix to maximize the Sharpe ratio, we get a well-diversified portfolio allocation with an expected annual return, volatility, and Sharpe ratio as shown above.
