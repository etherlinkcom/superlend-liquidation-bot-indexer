
- Borrow Event(address indexed reserve, address user, address indexed onBehalfOf, uint256 amount, uint8 interestRateMode, uint256 borrowRate, uint16 indexed referralCode):
  - topic0 :- 0xb3d084820fb1a9decffb176436bd02558d15fac9b0ddfed8c465bc7359d7dce0


- Database
  - Users Table
    - id -> INTEGER PRIMARY KEY AUTOINCREMENT
    - user_address -> TEXT UNIQUE
    - block_number -> INTEGER DEFAULT 0
    - health_factor -> REAL DEFAULT 0.0
    - timestamp -> DATETIME DEFAULT CURRENT_TIMESTAMP
  - Last Index Block Table (Just one row)
    - id -> INTEGER PRIMARY KEY AUTOINCREMENT
    - block_number -> INTEGER DEFAULT 0
    - timestamp -> DATETIME DEFAULT CURRENT_TIMESTAMP

  - Debt/Collateral Tables
    - id -> INTEGER PRIMARY KEY AUTOINCREMENT
    - user_address -> TEXT UNIQUE
    - reserve_address -> TEXT
    - block_number -> INTEGER DEFAULT 0
    - isCollateral -> BOOLEAN // True if it is collateral, False if it is debt
    - amount -> INTEGER
    - query -> SELECT * FROM table WHERE user_address = ? ORDER BY amount DESC
    - Result data
      | user_address | reserve_address | block_number | isCollateral | amount |
      |--------------|-----------------|--------------|--------------|--------|
      | user1        | reserve1        | 123          | True         | 200    |
      | user1        | reserve3        | 123          | True         | 150    |
      | user1        | reserve2        | 456          | False        | 100    |


- Reset helper
``` sql
DROP TABLE users;
```

currentATokenBalance (uint256) : collateral
currentVariableDebt (uint256) : debt

0.8 > 50% liquidation threshold

Time scale


