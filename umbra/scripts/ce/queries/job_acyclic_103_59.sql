select count(*) from imdb122, imdb13, imdb46 where imdb122.d = imdb13.s and imdb13.s = imdb46.s;