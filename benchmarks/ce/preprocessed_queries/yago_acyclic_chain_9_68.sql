select count(*) from yago50, yago36_1, yago5, yago22_3, yago22_4, yago22_5, yago21_6, yago21_7, yago36_8 where yago50.d = yago36_1.d and yago36_1.s = yago5.s and yago5.d = yago22_3.d and yago22_3.s = yago22_4.s and yago22_4.d = yago22_5.d and yago22_5.s = yago21_6.s and yago21_6.d = yago21_7.d and yago21_7.s = yago36_8.s;