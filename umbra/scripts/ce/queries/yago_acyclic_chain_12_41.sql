select count(*) from yago65, yago1, yago23, yago54 yago54_3, yago22, yago0, yago2 yago2_6, yago2 yago2_7, yago57 yago57_8, yago57 yago57_9, yago12, yago54 yago54_11 where yago65.d = yago1.s and yago1.d = yago0.d and yago23.s = yago54_3.s and yago23.d = yago22.d and yago54_3.d = yago54_11.d and yago0.s = yago2_6.d and yago2_6.s = yago2_7.s and yago2_7.d = yago57_8.s and yago57_8.d = yago57_9.d and yago57_9.s = yago12.d and yago12.s = yago54_11.s;