' stock.inc: what things cost

FUNCTION PriceOf(item AS STRING, quantity AS INTEGER) AS DOUBLE
    PriceOf = LEN(item) * 1.25 * quantity
END FUNCTION
