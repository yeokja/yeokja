---
date: "2024-02-10"
tags: ["markets"]
url: "https://www.stephendiehl.com/posts/slippage/"
---

# Slippage Modelling

When we're trading, we're not trading at the current market price, we're trading at the price the market will execute our order at. There's a difference between the price we see and the price we get. This difference is called *slippage*. Predicting a priori the slippage is an empirical problem, but we can model it and do better than just guessing or completely avoiding it in our model.

**Naive Slippage Model**

When we place an order on the market that is outsized relative to the size of other orders, our order will be executed at a price different to the price we see. However we can analytically model our expected slippage given the recent price history of the asset and then size our orders to minimise the impact of slippage. This is a function of three components.

The volatility of the asset is the main driver of slippage. The more volatile the asset, the more slippage we expect. We can model this by estimating the volatility of the asset from the recent price history and then using this to estimate the slippage. We can calculate the volatility using a simple rolling window average.

The other component of slippage is the *volume impact factor*. This is the amount the price will move due to the size of our order relative to the average daily volume. We can model this by estimating the average daily volume of the asset from the recent price history and then using this to estimate the slippage.

The third component of slippage is the *spread*. This is the difference between the bid and ask price. We can model this by estimating the spread from the recent price history of the asset. The total slippage can be modeled as the sum of these components:

$$S = S_{\text{base}} + S_{\text{volume}} + S_{\text{volatility}} + S_{\text{spread}}$$

where:

- $S_{\text{base}} = P \cdot \frac{m}{10000}$
- $S_{\text{volume}} = P \cdot f_v \cdot \frac{Q}{V}$
- $S_{\text{volatility}} = P \cdot \sigma \cdot f_v$
- $S_{\text{spread}} = s \cdot f_s$

Here:

- $P$ is the current price
- $f_v$ is the volume impact factor
- $f_s$ is the spread factor
- $Q$ is the order size
- $V$ is the average daily volume
- $\sigma$ is the volatility
- $s$ is the bid-ask spread
- $m$ is the minimum slippage in basis points

``` python
import numpy as np
import pandas as pd
from typing import Union, Optional

class SlippageModel:
    def __init__(
        self,
        volatility_window: int = 20,
        volume_impact_factor: float = 0.1,
        spread_factor: float = 0.5,
        min_slippage_bps: float = 1.0
    ):
        self.volatility_window = volatility_window
        self.volume_impact_factor = volume_impact_factor
        self.spread_factor = spread_factor
        self.min_slippage_bps = min_slippage_bps

    def estimate_slippage(
        self,
        price: float,
        order_size: float,
        avg_daily_volume: float,
        volatility: Optional[float] = None,
        bid_ask_spread: Optional[float] = None
    ) -> float:
        # Base slippage on minimum bps
        base_slippage = price * (self.min_slippage_bps / 10000)

        # Volume impact component
        volume_ratio = order_size / avg_daily_volume
        volume_impact = price * self.volume_impact_factor * volume_ratio

        # Volatility component (if provided)
        volatility_impact = 0
        if volatility is not None:
            volatility_impact = price * volatility * self.volume_impact_factor

        # Spread component
        spread_impact = 0
        if bid_ask_spread is not None:
            spread_impact = bid_ask_spread * self.spread_factor
        else:
            # Estimate spread as 0.1% of price if not provided
            spread_impact = price * 0.001 * self.spread_factor

        # Combine all components
        total_slippage = base_slippage + volume_impact + volatility_impact + spread_impact

        return total_slippage

    def calculate_volatility(self, prices: pd.Series) -> float:
        returns = prices.pct_change().dropna()
        volatility = returns.rolling(window=self.volatility_window).std()
        return volatility.iloc[-1]

    def estimate_trade_price(
        self,
        current_price: float,
        order_size: float,
        avg_daily_volume: float,
        is_buy: bool,
        volatility: Optional[float] = None,
        bid_ask_spread: Optional[float] = None
    ) -> float:
        slippage = self.estimate_slippage(
            current_price,
            order_size,
            avg_daily_volume,
            volatility,
            bid_ask_spread
        )

        # Add slippage for buys, subtract for sells
        execution_price = current_price + (slippage if is_buy else -slippage)
        return execution_price
```

Now to use this model we need to estimate the volatility and bid-ask spread of the asset. We'll use an equity-like asset for this example.

``` python
# Initialize model
model = SlippageModel(
    volatility_window=20,
    volume_impact_factor=0.1,
    spread_factor=0.5,
    min_slippage_bps=1.0
)

# Example trade
current_price = 100.0
order_size = 1000
avg_daily_volume = 1000000
volatility = 0.02  # 2% daily volatility
bid_ask_spread = 0.05

# Estimate slippage for a buy order
slippage = model.estimate_slippage(
    current_price,
    order_size,
    avg_daily_volume,
    volatility,
    bid_ask_spread
)

execution_price = model.estimate_trade_price(
    current_price,
    order_size,
    avg_daily_volume,
    is_buy=True,
    volatility=volatility,
    bid_ask_spread=bid_ask_spread
)

print(f"Current Price: ${current_price:.2f}")
print(f"Estimated Slippage: ${slippage:.4f}")
print(f"Execution Price: ${execution_price:.2f}")
print(f"Slippage Impact (bps): {(slippage/current_price)*10000:.2f}")
```

**Abdi and Ranaldo Model**

The Abdi and Ranaldo (2017) model offers a comprehensive framework for estimating bid-ask spreads in financial markets. The model is grounded in the concept of efficient price variance, which reflects the underlying volatility of asset prices. By analyzing high, low, and closing prices, the model aims to provide a more accurate estimation of the bid-ask spread. At the core of the model is the calculation of the average price, denoted as:

$$\eta_t = \frac{(h_t + h_l)}{2}$$

for each trading period. This average price serves as a reference point for assessing price movements. The model then examines the variance of changes in these average prices, represented as $\text{Var}(\Delta \eta)$, to capture the dynamics of price fluctuations over time. To estimate the efficient price variance, the model uses the variance of the changes in the average price, adjusted by a constant factor. The efficient price variance is calculated as:

$$\sigma^2_{\text{eff}} = \frac{\text{Var}(\Delta \eta)}{2 - \frac{k_1}{2}}$$

Where $k_1$ is a constant derived from the model's theoretical foundations (see the derivation in the original paper). This adjustment accounts for the impact of market microstructure on price formation. Finally, the model derives the bid-ask spread using the relationship between the variance of closing prices and the efficient price variance. The spread ($S$) is estimated as:

$$S = \sqrt{4 \left( \text{Var}(c_t - \eta_{avg}) - \left(0.5 + \frac{k_1}{8}\right) \sigma^2_{\text{eff}} \right)}$$

This equation shows us how the spread is influenced by both the observed price movements and the underlying market conditions, providing traders with a robust tool for slippage estimation. For more details, refer to the original paper [here](https://academic.oup.com/rfs/article/30/12/4437/4047344).

``` python
def abdi_ranaldo_spread_estimator(
    highs: pd.Series,
    lows: pd.Series,
    closes: pd.Series
) -> tuple[float, float]:
    k1 = 4 * np.log(2)  # Constant used in the model

    # Calculate η_t = (high + low)/2 for each day
    eta = (highs + lows) / 2

    # Calculate variance of η changes
    eta_changes = eta.diff().dropna()
    var_eta_changes = eta_changes.var()

    # Estimate efficient price variance
    efficient_variance = var_eta_changes / (2 - k1/2)

    # Calculate variance of close vs average etas
    eta_avg = (eta + eta.shift(-1)) / 2  # (η_t + η_{t+1})/2
    close_vs_eta = closes - eta_avg
    var_close_eta = close_vs_eta.dropna().var()

    # Solve for spread
    spread = np.sqrt(4 * (var_close_eta - (0.5 + k1/8) * efficient_variance))

    return spread, efficient_variance
```

Using this function we can create a model that estimates the bid-ask spread and then uses this to estimate the slippage for a trade using the same class interface as before.

``` python
class AbdiRanaldoModel(SlippageModel):
    def __init__(
        self,
        volatility_window: int = 20,
        volume_impact_factor: float = 0.1,
        spread_factor: float = 0.5,
        min_slippage_bps: float = 1.0
    ):
        super().__init__(volatility_window, volume_impact_factor, spread_factor, min_slippage_bps)

    def estimate_market_spread(
        self,
        highs: pd.Series,
        lows: pd.Series,
        closes: pd.Series
    ) -> float:
        """
        Estimates the bid-ask spread using the Abdi-Ranaldo method.
        """
        spread, _ = abdi_ranaldo_spread_estimator(highs, lows, closes)
        return spread
```

Now we can use this model to estimate the slippage for an individual trade from it's recent price history.

``` python
model = AbdiRanaldoModel()

# Estimate the market spread
estimated_spread = model.estimate_market_spread(
    data['High'],
    data['Low'],
    data['Close']
)

# Use the estimated spread in slippage calculation
slippage = model.estimate_slippage(
    price=data['Close'].iloc[-1],
    order_size=1000,
    avg_daily_volume=data['Volume'].iloc[-1],
    bid_ask_spread=estimated_spread
)

# Estimate the execution price
execution_price = model.estimate_trade_price(
    current_price=data['Close'].iloc[-1],
    order_size=1000,
    avg_daily_volume=data['Volume'].iloc[-1],
    is_buy=True,
    bid_ask_spread=estimated_spread
)
```
