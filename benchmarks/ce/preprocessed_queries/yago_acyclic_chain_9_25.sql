select count(*) from yago1_0, yago1_1, yago17_2, yago17_3, yago17_4, yago17_5, yago22, yago35_7, yago35_8 where yago1_0.d = yago1_1.d and yago1_1.s = yago17_2.d and yago17_2.s = yago17_3.s and yago17_3.d = yago17_4.d and yago17_4.s = yago17_5.s and yago17_5.d = yago22.d and yago22.s = yago35_7.s and yago35_7.d = yago35_8.d;