
- Borrow Event(address indexed reserve, address user, address indexed onBehalfOf, uint256 amount, uint8 interestRateMode, uint256 borrowRate, uint16 indexed referralCode):
  - topic0 :- 0xb3d084820fb1a9decffb176436bd02558d15fac9b0ddfed8c465bc7359d7dce0


- Database
  - Users Table
    - id -> INTEGER PRIMARY KEY AUTOINCREMENT
    - user_address -> TEXT UNIQUE
    - block_number -> INTEGER DEFAULT 0
    - health_factor -> REAL DEFAULT 0.0
    - timestamp -> DATETIME DEFAULT CURRENT_TIMESTAMP
    - totalDebtValueInUsd -> REAL DEFAULT 0.0
    - totalCollateralValueInUsd -> REAL DEFAULT 0.0
    - leadingCollateralReserve -> TEXT DEFAULT ""
    - leadingDebtReserve -> TEXT DEFAULT ""

  - Last Index Block Table (Just one row)
    - id -> INTEGER PRIMARY KEY AUTOINCREMENT
    - block_number -> INTEGER DEFAULT 0
    - timestamp -> DATETIME DEFAULT CURRENT_TIMESTAMP

  - User Debt/Collateral Tables
    - id -> INTEGER PRIMARY KEY AUTOINCREMENT
    - user_address -> TEXT NOT NULL
    - reserve_address -> TEXT NOT NULL
    - amount -> REAL DEFAULT 0.0
    - is_collateral -> BOOLEAN DEFAULT TRUE

  <!-- - Debt/Collateral Tables
    - id -> INTEGER PRIMARY KEY AUTOINCREMENT
    - user_address -> TEXT UNIQUE
    - reserve_address -> TEXT
    - block_number -> INTEGER DEFAULT 0
    - isCollateral -> BOOLEAN // True if it is collateral, False if it is debt
    - amountInUsd -> INTEGER
    - query -> SELECT * FROM table WHERE user_address = ? ORDER BY amount DESC
    - Result data
      | user_address | reserve_address | block_number | isCollateral | amountInUsd |
      |--------------|-----------------|--------------|--------------|-------------|
      | user1        | reserve1        | 123          | True         | 200USD      |
      | user1        | reserve3        | 123          | True         | 100USD      |
      | user1        | reserve2        | 456          | False        | 50USD       |
      | user1        | reserve5        | 456          | False        | 100USD      | -->


- Reset helper
``` sql
DROP TABLE users;
```

currentATokenBalance (uint256) : collateral
currentVariableDebt (uint256) : debt

 => < 0.9 -> 100%
=> 0.9 and < 1 -> 50%