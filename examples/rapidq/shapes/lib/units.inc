' units.inc: an area as text, with two decimals (the Shapes example).
FUNCTION AreaText (Area AS DOUBLE) AS STRING
    Result = FORMAT$("%.2f", Area) + " square cm"
END FUNCTION
