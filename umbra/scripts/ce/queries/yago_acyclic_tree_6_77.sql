select count(*) from yago36 yago36_0, yago36 yago36_1, yago36 yago36_2, yago25, yago36 yago36_4, yago21 where yago36_0.s = yago36_1.s and yago36_1.s = yago36_2.s and yago36_2.s = yago25.s and yago36_1.d = yago36_4.d and yago36_4.s = yago21.s;