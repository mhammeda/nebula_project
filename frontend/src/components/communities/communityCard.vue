<template>
    <v-hover v-slot:default="{ hover }">
        <v-card class="mx-auto" color="grey lighten-4">
            <v-card-title>
                {{ item.title }}
            </v-card-title>
            <v-card-subtitle class="text-left">
                {{ item.id }}
            </v-card-subtitle>
            <v-expand-transition>
                <div
                    v-if="hover"
                    class="d-flex transition-fast-in-fast-out v-card--reveal primary white--text"
                    style="height: 100%;"
                >
                    <v-card-text class="text-left">
                        <p><b>Description:</b></p>
                        <p>{{ item.description }}</p>
                        <v-btn
                            absolute
                            color="secondary"
                            class="white--text"
                            small
                            right
                            top
                            @click="visitCommunityPage"
                        >
                            View
                        </v-btn>
                    </v-card-text>
                </div>
            </v-expand-transition>
        </v-card>
    </v-hover>
</template>
<script>
export default {
    props: ['item'],
    methods: {
        visitCommunityPage() {
            const url = `/community/${this.item.id}`
            this.$router.push(url)
        },
    },
}
</script>
