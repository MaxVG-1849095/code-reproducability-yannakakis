select count(*) from yago5, yago17_1, yago2, yago21_3, yago39, yago36_5, yago44, yago36_7, yago36_8, yago21_9, yago17_10, yago17_11 where yago5.d = yago21_3.d and yago17_1.s = yago17_11.d and yago17_1.d = yago2.d and yago21_3.s = yago39.s and yago39.d = yago36_5.d and yago36_5.s = yago44.d and yago44.s = yago36_7.s and yago36_7.d = yago36_8.d and yago36_8.s = yago21_9.s and yago21_9.d = yago17_10.s and yago17_10.d = yago17_11.s;